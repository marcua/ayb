pub mod paths;
mod sqlite;

use crate::error::StacksError;
use crate::hosted_db::sqlite::run_sqlite_query;
use crate::stacks_db::models::DBType;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::vec::Vec;

#[derive(Serialize, Debug, Deserialize)]
pub struct QueryResult {
    pub fields: Vec<String>,
    pub rows: Vec<Vec<String>>,
}

pub fn run_query(
    path: &PathBuf,
    query: &str,
    db_type: &DBType,
) -> Result<QueryResult, StacksError> {
    match db_type {
        DBType::Sqlite => Ok(run_sqlite_query(path, query)?),
        _ => {
            return Err(StacksError {
                message: "Unsupported DB type".to_string(),
            })
        }
    }
}
