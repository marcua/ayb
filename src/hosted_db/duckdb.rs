use crate::error::AybError;
use crate::hosted_db::engine::DbEngine;
use crate::hosted_db::{QueryMode, QueryResult};
use crate::server::config::AybConfigSnapshots;
use duckdb::types::Value;
use std::path::{Path, PathBuf};

pub struct DuckdbEngine;

impl DbEngine for DuckdbEngine {
    fn query(
        &self,
        path: &Path,
        query: &str,
        params: &[serde_json::Value],
        allow_unsafe: bool,
        query_mode: QueryMode,
    ) -> Result<QueryResult, AybError> {
        query_duckdb(&path.to_path_buf(), query, params, allow_unsafe, query_mode)
    }

    fn create_snapshot(
        &self,
        _config: &AybConfigSnapshots,
        db_path: &Path,
        snapshot_path: &Path,
    ) -> Result<(), AybError> {
        // Copy the database into a fresh snapshot file via an in-memory
        // connection that attaches both sides with explicit aliases.
        // Opening the source directly and running "COPY FROM DATABASE main"
        // fails with `Catalog "main" does not exist` because the source
        // catalog is named after the file, not "main". Opening the source
        // read-only would also make the whole instance (including the
        // attached destination) read-only, breaking the COPY. With an
        // in-memory read-write main, the source is attached READ_ONLY (so
        // it is never modified) and the destination read-write.
        let config = duckdb::Config::default()
            .threads(1)
            .map_err(config_err)?
            .max_memory("128MB")
            .map_err(config_err)?;
        let conn = duckdb::Connection::open_in_memory_with_flags(config)?;
        conn.execute_batch(&format!(
            "ATTACH '{}' AS src (READ_ONLY); ATTACH '{}' AS dst; \
             COPY FROM DATABASE src TO dst;",
            db_path.display(),
            snapshot_path.display()
        ))
        .map_err(map_duckdb_error)?;
        drop(conn);
        let result = query_duckdb(
            &snapshot_path.to_path_buf(),
            "SELECT count(*) FROM information_schema.tables;",
            &[],
            false,
            QueryMode::ReadOnly,
        )?;
        if result.rows.is_empty() {
            return Err(AybError::SnapshotError {
                message: "Snapshot verification failed: could not read snapshot".to_string(),
            });
        }
        Ok(())
    }

    fn db_type_str(&self) -> &'static str {
        "duckdb"
    }
}

fn query_duckdb(
    path: &PathBuf,
    query: &str,
    params: &[serde_json::Value],
    allow_unsafe: bool,
    query_mode: QueryMode,
) -> Result<QueryResult, AybError> {
    // Cap threads and memory on the Config *before* opening. DuckDB probes
    // the host at instantiation (/sys/devices/system/cpu/online,
    // /sys/fs/cgroup/..., /proc/self/*) to auto-size these to the whole
    // machine -- on a CI runner or large host that means many worker
    // threads and a multi-GB buffer pool, which blows the daemon's 256 MB
    // RLIMIT_AS. Explicit values override the auto-detected ones so DuckDB
    // stays within the sandbox budget regardless of host size. (The probe
    // paths themselves are allowed read-only in the Landlock ruleset; see
    // src/hosted_db/sandbox.rs -- without that the probe aborts the
    // process before any query runs.)
    let config = duckdb::Config::default()
        .access_mode(match query_mode {
            QueryMode::ReadOnly => duckdb::AccessMode::ReadOnly,
            QueryMode::ReadWrite => duckdb::AccessMode::ReadWrite,
        })
        .map_err(config_err)?
        .threads(1)
        .map_err(config_err)?
        .max_memory("128MB")
        .map_err(config_err)?;

    let conn = duckdb::Connection::open_with_flags(path, config)?;

    if !allow_unsafe {
        // Disable extension install/load and external (file/network)
        // access, then lock the configuration so a query can't re-enable
        // them. This is the same safety perimeter as SQLite's ATTACH/
        // defensive settings.
        conn.execute_batch(
            "SET autoinstall_known_extensions=false;
             SET autoload_known_extensions=false;
             SET enable_external_access=false;
             SET lock_configuration=true;",
        )?;
    }

    let mut prepared = conn.prepare(query).map_err(map_duckdb_error)?;

    let bound = params
        .iter()
        .map(json_to_duckdb_value)
        .collect::<Result<Vec<_>, _>>()?;
    let mut rows = prepared
        .query(duckdb::params_from_iter(bound))
        .map_err(map_duckdb_error)?;

    let num_columns = rows.as_ref().unwrap().column_count();
    let mut fields: Vec<String> = Vec::new();
    for i in 0..num_columns {
        fields.push(rows.as_ref().unwrap().column_name(i)?.to_string());
    }

    let mut results: Vec<Vec<Option<String>>> = Vec::new();
    while let Some(row) = rows.next().map_err(map_duckdb_error)? {
        let mut result: Vec<Option<String>> = Vec::new();
        for col_idx in 0..num_columns {
            let value: duckdb::Result<Value> = row.get(col_idx);
            match value {
                Ok(val) => result.push(duckdb_value_to_string(val)),
                Err(_) => result.push(None),
            }
        }
        results.push(result);
    }
    Ok(QueryResult {
        fields,
        rows: results,
    })
}

/// Convert a JSON-native bind parameter into a DuckDB value. Integers
/// bind as `BigInt`, non-integer numbers as `Double`, and booleans use
/// DuckDB's native `BOOLEAN` (unlike SQLite, which has no boolean). JSON
/// arrays and objects have no scalar DuckDB equivalent and are rejected.
fn json_to_duckdb_value(value: &serde_json::Value) -> Result<Value, AybError> {
    Ok(match value {
        serde_json::Value::Null => Value::Null,
        serde_json::Value::Bool(b) => Value::Boolean(*b),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Value::BigInt(i)
            } else if let Some(f) = n.as_f64() {
                Value::Double(f)
            } else {
                return Err(AybError::QueryError {
                    message: format!("Unsupported numeric bind parameter: {n}"),
                });
            }
        }
        serde_json::Value::String(s) => Value::Text(s.clone()),
        serde_json::Value::Array(_) | serde_json::Value::Object(_) => {
            return Err(AybError::QueryError {
                message: "Array and object bind parameters are not supported".to_string(),
            });
        }
    })
}

fn config_err(e: duckdb::Error) -> AybError {
    AybError::Other {
        message: format!("DuckDB config error: {e}"),
    }
}

fn map_duckdb_error(err: duckdb::Error) -> AybError {
    match &err {
        duckdb::Error::DuckDBFailure(_, Some(msg))
            if msg.contains("read-only") || msg.contains("Cannot execute write") =>
        {
            AybError::NoWriteAccessError {
                message: "Attempted to write to database while in read-only mode".to_string(),
            }
        }
        _ => AybError::from(err),
    }
}

fn duckdb_value_to_string(value: Value) -> Option<String> {
    match value {
        Value::Null => None,
        Value::Boolean(b) => Some(b.to_string()),
        Value::TinyInt(i) => Some(i.to_string()),
        Value::SmallInt(i) => Some(i.to_string()),
        Value::Int(i) => Some(i.to_string()),
        Value::BigInt(i) => Some(i.to_string()),
        Value::HugeInt(i) => Some(i.to_string()),
        Value::UTinyInt(i) => Some(i.to_string()),
        Value::USmallInt(i) => Some(i.to_string()),
        Value::UInt(i) => Some(i.to_string()),
        Value::UBigInt(i) => Some(i.to_string()),
        Value::Float(f) => Some(f.to_string()),
        Value::Double(f) => Some(f.to_string()),
        Value::Text(s) => Some(s),
        Value::Blob(b) => Some(String::from_utf8_lossy(&b).to_string()),
        Value::Timestamp(_, _) => Some(format!("{value:?}")),
        Value::Date32(d) => Some(d.to_string()),
        Value::Time64(_, t) => Some(t.to_string()),
        _ => Some(format!("{value:?}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_duckdb_create_insert_select() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.duckdb");

        let r = query_duckdb(
            &path,
            "CREATE TABLE t(id INTEGER, name VARCHAR);",
            &[],
            false,
            QueryMode::ReadWrite,
        )
        .unwrap();
        assert!(r.rows.is_empty());

        let r = query_duckdb(
            &path,
            "INSERT INTO t VALUES (1, 'hello'), (2, 'world');",
            &[],
            false,
            QueryMode::ReadWrite,
        )
        .unwrap();
        assert_eq!(r.fields, vec!["Count"]);
        assert_eq!(r.rows, vec![vec![Some("2".to_string())]]);

        let r = query_duckdb(
            &path,
            "SELECT * FROM t ORDER BY id;",
            &[],
            false,
            QueryMode::ReadOnly,
        )
        .unwrap();
        assert_eq!(r.fields, vec!["id", "name"]);
        assert_eq!(r.rows.len(), 2);
        assert_eq!(
            r.rows[0],
            vec![Some("1".to_string()), Some("hello".to_string())]
        );
        assert_eq!(
            r.rows[1],
            vec![Some("2".to_string()), Some("world".to_string())]
        );

        fs::remove_dir_all(dir.path()).ok();
    }

    #[test]
    fn test_duckdb_read_only_prevents_writes() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test_ro.duckdb");

        query_duckdb(
            &path,
            "CREATE TABLE t(id INTEGER);",
            &[],
            false,
            QueryMode::ReadWrite,
        )
        .unwrap();

        let result = query_duckdb(
            &path,
            "INSERT INTO t VALUES (1);",
            &[],
            false,
            QueryMode::ReadOnly,
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_duckdb_positional_params() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test_params.duckdb");

        query_duckdb(
            &path,
            "CREATE TABLE t(id INTEGER, name VARCHAR, score DOUBLE);",
            &[],
            false,
            QueryMode::ReadWrite,
        )
        .unwrap();

        query_duckdb(
            &path,
            "INSERT INTO t VALUES (1, 'alice', 9.5), (2, 'bob', 7.0);",
            &[],
            false,
            QueryMode::ReadWrite,
        )
        .unwrap();

        // Bind a string and a number positionally ($1, $2).
        let r = query_duckdb(
            &path,
            "SELECT id FROM t WHERE name = $1 AND score >= $2 ORDER BY id;",
            &[serde_json::json!("alice"), serde_json::json!(9.0)],
            false,
            QueryMode::ReadOnly,
        )
        .unwrap();
        assert_eq!(r.rows, vec![vec![Some("1".to_string())]]);

        // A NULL bind parameter compares as expected.
        let r = query_duckdb(
            &path,
            "SELECT count(*) FROM t WHERE $1 IS NULL;",
            &[serde_json::Value::Null],
            false,
            QueryMode::ReadOnly,
        )
        .unwrap();
        assert_eq!(r.rows, vec![vec![Some("2".to_string())]]);

        fs::remove_dir_all(dir.path()).ok();
    }
}
