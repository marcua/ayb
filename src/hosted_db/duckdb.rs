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
        allow_unsafe: bool,
        query_mode: QueryMode,
    ) -> Result<QueryResult, AybError> {
        query_duckdb(&path.to_path_buf(), query, allow_unsafe, query_mode)
    }

    fn create_snapshot(
        &self,
        _config: &AybConfigSnapshots,
        db_path: &Path,
        snapshot_path: &Path,
    ) -> Result<(), AybError> {
        let copy_query = format!(
            "ATTACH '{}' AS snapshot_dest;COPY FROM DATABASE main TO snapshot_dest;",
            snapshot_path.display()
        );
        let result = query_duckdb(
            &db_path.to_path_buf(),
            &copy_query,
            true,
            QueryMode::ReadOnly,
        )?;
        if !result.rows.is_empty() {
            return Err(AybError::SnapshotError {
                message: format!("Unexpected snapshot result: {result:?}"),
            });
        }
        let result = query_duckdb(
            &snapshot_path.to_path_buf(),
            "SELECT count(*) FROM information_schema.tables;",
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

    let mut rows = prepared.query([]).map_err(map_duckdb_error)?;

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
            false,
            QueryMode::ReadWrite,
        )
        .unwrap();
        assert!(r.rows.is_empty());

        let r = query_duckdb(
            &path,
            "INSERT INTO t VALUES (1, 'hello'), (2, 'world');",
            false,
            QueryMode::ReadWrite,
        )
        .unwrap();
        assert_eq!(r.fields, vec!["Count"]);
        assert_eq!(r.rows, vec![vec![Some("2".to_string())]]);

        let r = query_duckdb(
            &path,
            "SELECT * FROM t ORDER BY id;",
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
            false,
            QueryMode::ReadWrite,
        )
        .unwrap();

        let result = query_duckdb(
            &path,
            "INSERT INTO t VALUES (1);",
            false,
            QueryMode::ReadOnly,
        );
        assert!(result.is_err());
    }
}
