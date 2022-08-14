mod sqlite;

use std::fmt;
use std::path::PathBuf;
use std::vec::Vec;
use clap::ValueEnum;
use crate::databases::sqlite::run_sqlite_query;

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

pub struct QueryResult {
    fields: Vec<String>,
    rows: Vec<Vec<String>>,
}

pub fn run_query(path: &PathBuf, query: &str, db_type: &DBType) -> Result<(), &'static str> {
    let query_results;
    match db_type {
        DBType::Sqlite => {
            query_results = run_sqlite_query(path, query);
        }
        _ => {
            return Err("Unsupported DB type")
        }
    }
    match query_results {
        Ok(result) => {
            println!("Result schema: {:#?}", result.fields);
            println!("Results: {:#?}", result.rows);
        }
        Err(err) => {
            println!("SQLite error: {}", err);
        }
    }
    Ok(())
}
