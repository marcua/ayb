mod sqlite;

use crate::hosted_db::sqlite::run_sqlite_query;
use clap::ValueEnum;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::path::PathBuf;
use std::vec::Vec;

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
pub enum DBType {
    Sqlite,
    Duckdb,
}

impl fmt::Display for DBType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Serialize, Deserialize)]
pub struct QueryResult {
    pub fields: Vec<String>,
    pub rows: Vec<Vec<String>>,
}

pub fn run_query(path: &PathBuf, query: &str, db_type: &DBType) -> Result<QueryResult, String> {
    match db_type {
        DBType::Sqlite => match run_sqlite_query(path, query) {
            Ok(result) => Ok(result),
            Err(err) => Err(format!("SQLite error: {}", err)),
        },
        _ => return Err("Error: Unsupported DB type".to_string()),
    }
}
