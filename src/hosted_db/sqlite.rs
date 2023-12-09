use crate::error::AybError;
use crate::hosted_db::QueryResult;
use crate::http::structs::AybConfigIsolation;
use ayb_hosted_db_runner::query_sqlite;
use std::path::PathBuf;

pub fn run_sqlite_query(
    path: &PathBuf,
    query: &str,
    isolation: &Option<AybConfigIsolation>,
) -> Result<QueryResult, AybError> {
    match isolation {
        Some(isolation) => Ok(QueryResult {
            fields: Vec::new(),
            rows: Vec::new(),
        }),
        None => Ok(query_sqlite(path, query)?.into()),
    }
}
