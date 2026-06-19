use crate::error::AybError;
use crate::hosted_db::engine::DbEngine;
use crate::hosted_db::{QueryMode, QueryResult};
use crate::server::config::AybConfigSnapshots;
use rusqlite;
use rusqlite::config::DbConfig;
use rusqlite::limits::Limit;
use rusqlite::types::ValueRef;
use std::path::{Path, PathBuf};

pub struct SqliteEngine;

impl DbEngine for SqliteEngine {
    fn query(
        &self,
        path: &Path,
        query: &str,
        params: &[serde_json::Value],
        allow_unsafe: bool,
        query_mode: QueryMode,
    ) -> Result<QueryResult, AybError> {
        query_sqlite(&path.to_path_buf(), query, params, allow_unsafe, query_mode)
    }

    fn create_snapshot(
        &self,
        _config: &AybConfigSnapshots,
        db_path: &Path,
        snapshot_path: &Path,
    ) -> Result<(), AybError> {
        let backup_query = format!("VACUUM INTO \"{}\"", snapshot_path.display());
        let result = query_sqlite(
            &db_path.to_path_buf(),
            &backup_query,
            &[],
            true,
            QueryMode::ReadOnly,
        )?;
        if !result.rows.is_empty() {
            return Err(AybError::SnapshotError {
                message: format!("Unexpected snapshot result: {result:?}"),
            });
        }
        let result = query_sqlite(
            &snapshot_path.to_path_buf(),
            "PRAGMA integrity_check;",
            &[],
            false,
            QueryMode::ReadOnly,
        )?;
        if result.fields.len() != 1
            || result.rows.len() != 1
            || result.rows[0][0] != Some("ok".to_string())
        {
            return Err(AybError::SnapshotError {
                message: format!("Snapshot failed integrity check: {result:?}"),
            });
        }
        Ok(())
    }

    fn db_type_str(&self) -> &'static str {
        "sqlite"
    }
}

/// `allow_unsafe` disables features that prevent abuse but also
/// prevent backups/snapshots. The only known use case in the codebase
/// is for snapshots.
fn query_sqlite(
    path: &PathBuf,
    query: &str,
    params: &[serde_json::Value],
    allow_unsafe: bool,
    query_mode: QueryMode,
) -> Result<QueryResult, AybError> {
    // The flags below are the default `open` flags in `rusqlite`
    // except for `..READ_ONLY` and `..READ_WRITE`.
    let mut open_flags =
        rusqlite::OpenFlags::SQLITE_OPEN_URI | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX;
    open_flags |= match query_mode {
        QueryMode::ReadOnly => rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
        QueryMode::ReadWrite => {
            rusqlite::OpenFlags::SQLITE_OPEN_CREATE | rusqlite::OpenFlags::SQLITE_OPEN_READ_WRITE
        }
    };
    let conn = rusqlite::Connection::open_with_flags(path, open_flags)?;

    // Set busy timeout to 5 seconds to handle concurrent access
    conn.pragma_update(None, "busy_timeout", 5000)?;

    // Configure SQLite for optimal ayb usage
    if matches!(query_mode, QueryMode::ReadWrite) {
        // Enable WAL (Write-Ahead Logging) mode for better concurrency and performance.
        // This operation is idempotent and will convert non-WAL DBs to WAL ones.
        let _mode: String = conn.query_row("PRAGMA journal_mode=WAL", [], |row| row.get(0))?;

        // Set synchronous mode to FULL for maximum durability
        conn.pragma_update(None, "synchronous", "FULL")?;

        // Enable foreign key constraints
        conn.pragma_update(None, "foreign_keys", true)?;
    }

    if !allow_unsafe {
        // Disable the usage of ATTACH
        // https://www.sqlite.org/lang_attach.html
        conn.set_limit(Limit::SQLITE_LIMIT_ATTACHED, 0);
        // Prevent queries from deliberately corrupting the database
        // https://www.sqlite.org/c3ref/c_dbconfig_defensive.html
        conn.db_config(DbConfig::SQLITE_DBCONFIG_DEFENSIVE)?;
    }

    let mut prepared = conn.prepare(query)?;
    let num_columns = prepared.column_count();
    let mut fields: Vec<String> = Vec::new();
    for column_index in 0..num_columns {
        fields.push(String::from(prepared.column_name(column_index)?))
    }

    let bound = params
        .iter()
        .map(json_to_sqlite_value)
        .collect::<Result<Vec<_>, _>>()?;
    let mut rows = prepared.query(rusqlite::params_from_iter(bound))?;
    let mut results: Vec<Vec<Option<String>>> = Vec::new();
    while let Some(row) = rows.next().map_err(|err| match err {
        rusqlite::Error::SqliteFailure(ref code, _)
            if code.code == rusqlite::ErrorCode::ReadOnly && code.extended_code == 8 =>
        {
            AybError::NoWriteAccessError {
                message: "Attempted to write to database while in read-only mode".to_string(),
            }
        }
        _ => AybError::from(err),
    })? {
        let mut result: Vec<Option<String>> = Vec::new();
        for column_index in 0..num_columns {
            let column_value = row.get_ref(column_index)?;
            result.push(match column_value {
                ValueRef::Null => None,
                ValueRef::Integer(i) => Some(i.to_string()),
                ValueRef::Real(f) => Some(f.to_string()),
                ValueRef::Text(_t) => Some(column_value.as_str()?.to_string()),
                ValueRef::Blob(_b) => {
                    Some(std::str::from_utf8(column_value.as_bytes()?)?.to_string())
                }
            });
        }
        results.push(result);
    }
    Ok(QueryResult {
        fields,
        rows: results,
    })
}

/// Convert a JSON-native bind parameter into a SQLite value. SQLite has
/// no native boolean, so `true`/`false` bind as integer `1`/`0`. JSON
/// arrays and objects have no scalar SQLite equivalent and are rejected.
fn json_to_sqlite_value(value: &serde_json::Value) -> Result<rusqlite::types::Value, AybError> {
    use rusqlite::types::Value;
    Ok(match value {
        serde_json::Value::Null => Value::Null,
        serde_json::Value::Bool(b) => Value::Integer(*b as i64),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Value::Integer(i)
            } else if let Some(f) = n.as_f64() {
                Value::Real(f)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sqlite_positional_params() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test_params.sqlite");

        query_sqlite(
            &path,
            "CREATE TABLE t(id INTEGER, name TEXT, score REAL);",
            &[],
            false,
            QueryMode::ReadWrite,
        )
        .unwrap();

        query_sqlite(
            &path,
            "INSERT INTO t VALUES (1, 'alice', 9.5), (2, 'bob', 7.0);",
            &[],
            false,
            QueryMode::ReadWrite,
        )
        .unwrap();

        // Bind a string and a number positionally (?1, ?2).
        let r = query_sqlite(
            &path,
            "SELECT id FROM t WHERE name = ?1 AND score >= ?2 ORDER BY id;",
            &[serde_json::json!("alice"), serde_json::json!(9.0)],
            false,
            QueryMode::ReadOnly,
        )
        .unwrap();
        assert_eq!(r.rows, vec![vec![Some("1".to_string())]]);

        // A NULL bind parameter compares as expected.
        let r = query_sqlite(
            &path,
            "SELECT count(*) FROM t WHERE ?1 IS NULL;",
            &[serde_json::Value::Null],
            false,
            QueryMode::ReadOnly,
        )
        .unwrap();
        assert_eq!(r.rows, vec![vec![Some("2".to_string())]]);

        // Booleans bind as SQLite integers (1/0).
        let r = query_sqlite(
            &path,
            "SELECT ?1;",
            &[serde_json::json!(true)],
            false,
            QueryMode::ReadOnly,
        )
        .unwrap();
        assert_eq!(r.rows, vec![vec![Some("1".to_string())]]);

        // Array/object parameters are rejected.
        let err = query_sqlite(
            &path,
            "SELECT ?1;",
            &[serde_json::json!([1, 2, 3])],
            false,
            QueryMode::ReadOnly,
        );
        assert!(err.is_err());
    }
}
