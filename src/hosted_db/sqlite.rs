use crate::error::AybError;
use crate::hosted_db::QueryResult;
use rusqlite;
use std::path::PathBuf;

pub fn run_sqlite_query(path: &PathBuf, query: &str) -> Result<QueryResult, AybError> {
    let conn = rusqlite::Connection::open(path)?;
    let mut prepared = conn.prepare(query)?;
    let num_columns = prepared.column_count();
    let mut fields: Vec<String> = Vec::new();
    for column_index in 0..num_columns {
        fields.push(String::from(prepared.column_name(column_index)?))
    }

    let mut rows = prepared.query([])?;
    let mut results: Vec<Vec<String>> = Vec::new();
    while let Some(row) = rows.next()? {
        let mut result: Vec<String> = Vec::new();
        for column_index in 0..num_columns {
            result.push(row.get(column_index)?);
        }
        results.push(result);
    }
    Ok(QueryResult {
        fields,
        rows: results,
    })
}
