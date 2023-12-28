use crate::error::AybError;
use crate::hosted_db::{sandbox::run_in_sandbox, QueryResult};
use crate::http::structs::AybConfigIsolation;
use rusqlite::config::DbConfig;
use rusqlite::limits::Limit;
use rusqlite::types::ValueRef;
use serde_json;
use std::path::{Path, PathBuf};

pub fn query_sqlite(path: &PathBuf, query: &str) -> Result<QueryResult, AybError> {
    let conn = rusqlite::Connection::open(path)?;

    // Disable the usage of ATTACH
    // https://www.sqlite.org/lang_attach.html
    conn.set_limit(Limit::SQLITE_LIMIT_ATTACHED, 0);
    // Prevent queries from deliberately corrupting the database
    // https://www.sqlite.org/c3ref/c_dbconfig_defensive.html
    conn.db_config(DbConfig::SQLITE_DBCONFIG_DEFENSIVE)?;

    let mut prepared = conn.prepare(query)?;
    let num_columns = prepared.column_count();
    let mut fields: Vec<String> = Vec::new();
    for column_index in 0..num_columns {
        fields.push(String::from(prepared.column_name(column_index)?))
    }

    let mut rows = prepared.query([])?;
    let mut results: Vec<Vec<Option<String>>> = Vec::new();
    while let Some(row) = rows.next()? {
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
) -> Result<QueryResult, AybError> {
    if let Some(isolation) = isolation {
        let result = run_in_sandbox(Path::new(&isolation.nsjail_path), path, query).await?;

        if !result.stderr.is_empty() {
            let error: AybError = serde_json::from_str(&result.stderr)?;
            return Err(error);
        } else if result.status != 0 {
            return Err(AybError::Other {
                message: format!(
                    "Error status from sandboxed query runner: {}",
                    result.status
                ),
            });
        } else if result.stdout.len() > 0 {
            let query_result: QueryResult = serde_json::from_str(&result.stdout)?;
            return Ok(query_result);
        } else {
            return Err(AybError::Other {
                message: "No results from sandboxed query runner".to_string(),
            });
        }
    }

    // No isolation configuration, so run the query without a sandbox.
    Ok(query_sqlite(path, query)?)
}
