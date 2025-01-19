use crate::error::AybError;
use crate::hosted_db::{sandbox::run_in_sandbox, QueryMode, QueryResult};
use crate::server::config::AybConfigIsolation;
use rusqlite;
use rusqlite::config::DbConfig;
use rusqlite::limits::Limit;
use rusqlite::types::ValueRef;
use serde_json;
use std::path::{Path, PathBuf};

/// `allow_unsafe` disables features that prevent abuse but also
/// prevent backups/snapshots. The only known use case in the codebase
/// is for snapshots.
pub fn query_sqlite(
    path: &PathBuf,
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

/// If isolation is configured, run `query` against the database at
/// `path` using the `isolation` settings.
pub async fn potentially_isolated_sqlite_query(
    path: &PathBuf,
    query: &str,
    isolation: &Option<AybConfigIsolation>,
    query_mode: QueryMode,
) -> Result<QueryResult, AybError> {
    if let Some(isolation) = isolation {
        println!("potentially1");
        let result =
            run_in_sandbox(Path::new(&isolation.nsjail_path), path, query, query_mode).await?;
        println!("potentially2");
        if !result.stderr.is_empty() {
            println!("potentially3");
            // Before shipping, consider whether to still try to parse and then catch the parsing error.
            return Err(AybError::QueryError {
                message: format!(
                    "Error message from sandboxed query runner: {}",
                    result.stderr
                ),
            });
        } else if result.status != 0 {
            println!("potentially5");
            return Err(AybError::QueryError {
                message: format!(
                    "Error status from sandboxed query runner: {}",
                    result.status
                ),
            });
        } else if !result.stdout.is_empty() {
            println!("potentially6");
            let query_result: QueryResult = serde_json::from_str(&result.stdout)?;
            println!("potentially7");
            return Ok(query_result);
        } else {
            println!("potentially8");
            return Err(AybError::QueryError {
                message: "No results from sandboxed query runner".to_string(),
            });
        }
    }

    // No isolation configuration, so run the query without a sandbox.
    query_sqlite(path, query, false, query_mode)
}
