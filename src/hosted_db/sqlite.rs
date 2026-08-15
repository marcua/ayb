use crate::error::AybError;
use crate::hosted_db::engine::DbEngine;
use crate::hosted_db::{sql_string_literal, QueryMode, QueryResult};
use rusqlite;
use rusqlite::config::DbConfig;
use rusqlite::limits::Limit;
use rusqlite::types::ValueRef;
use std::path::Path;

pub struct SqliteEngine;

impl DbEngine for SqliteEngine {
    fn query(
        &self,
        path: &Path,
        query: &str,
        query_mode: QueryMode,
    ) -> Result<QueryResult, AybError> {
        query_sqlite(path, query, false, query_mode)
    }

    fn create_snapshot(&self, db_path: &Path, snapshot_path: &Path) -> Result<(), AybError> {
        // The snapshot path embeds user-controlled entity and database
        // slugs, so it is rendered as an escaped SQL string literal
        // rather than interpolated raw. (Single quotes, not the double
        // quotes SQLite would read as an identifier.)
        let backup_query = format!("VACUUM INTO {}", sql_string_literal(snapshot_path));
        let result = query_sqlite(db_path, &backup_query, true, QueryMode::ReadOnly)?;
        if !result.rows.is_empty() {
            return Err(AybError::SnapshotError {
                message: format!("Unexpected snapshot result: {result:?}"),
            });
        }
        // Verify the copy we just wrote with the same check an uploaded
        // file gets, so there is one definition of "this is a readable
        // SQLite database" for snapshots, exports, and imports alike.
        self.validate(snapshot_path)
            .map_err(|err| AybError::SnapshotError {
                message: format!("Snapshot failed verification: {err}"),
            })
    }

    fn validate(&self, path: &Path) -> Result<(), AybError> {
        // The connection is opened read-only with the standard perimeter
        // (no ATTACH, defensive mode), so an untrusted file cannot use it
        // to reach outside itself.
        match query_sqlite(path, "PRAGMA integrity_check;", false, QueryMode::ReadOnly) {
            Ok(result)
                if result.fields.len() == 1
                    && result.rows.len() == 1
                    && result.rows[0][0] == Some("ok".to_string()) =>
            {
                Ok(())
            }
            Ok(result) => Err(AybError::Other {
                message: format!("File failed SQLite integrity check: {result:?}"),
            }),
            Err(err) => Err(AybError::Other {
                message: format!("File is not a valid SQLite database: {err}"),
            }),
        }
    }
}

/// `allow_unsafe` disables features that prevent abuse but also
/// prevent backups/snapshots. The only known use case in the codebase
/// is for snapshots.
fn query_sqlite(
    path: &Path,
    query: &str,
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

    let mut rows = prepared.query([])?;
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
