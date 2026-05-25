pub mod daemon_registry;
pub mod duckdb;
pub mod engine;
pub mod paths;
pub mod sandbox;
pub mod sqlite;

use crate::ayb_db::models::DBType;
use crate::error::AybError;
use crate::formatting::TabularFormatter;
use crate::from_str;
use crate::hosted_db::duckdb::DuckdbEngine;
use crate::hosted_db::engine::DbEngine;
use crate::hosted_db::sqlite::SqliteEngine;
use crate::try_from_i16;
use prettytable::{Cell, Row, Table};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::str::FromStr;
use std::vec::Vec;

#[derive(Serialize, Debug, Deserialize, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
#[repr(i16)]
pub enum QueryMode {
    ReadOnly = 0,
    ReadWrite = 1,
}

try_from_i16!(QueryMode, {
    0 => QueryMode::ReadOnly,
    1 => QueryMode::ReadWrite
});

from_str!(QueryMode, {
    "read-only" => QueryMode::ReadOnly,
    "read-write" => QueryMode::ReadWrite
});

impl QueryMode {
    pub fn to_str(&self) -> &str {
        match self {
            QueryMode::ReadOnly => "read-only",
            QueryMode::ReadWrite => "read-write",
        }
    }

    /// Returns true if this access level is sufficient for the requested level.
    pub fn permits(&self, requested: QueryMode) -> bool {
        *self >= requested
    }
}

#[derive(Serialize, Debug, Deserialize)]
pub struct QueryResult {
    pub fields: Vec<String>,
    pub rows: Vec<Vec<Option<String>>>,
}

impl TabularFormatter for QueryResult {
    fn to_table(&self) -> Table {
        let mut table = Table::new();
        table.set_titles(Row::new(
            self.fields
                .iter()
                .map(|cell| Cell::new(cell))
                .collect::<Vec<_>>(),
        ));
        for row in &self.rows {
            let cells = row
                .iter()
                .map(|cell| {
                    Cell::new(match cell {
                        Some(s) => s,
                        None => "NULL",
                    })
                })
                .collect::<Vec<_>>();
            table.add_row(Row::new(cells));
        }
        table
    }
}

pub fn engine_for(db_type: &DBType) -> Box<dyn DbEngine> {
    match db_type {
        DBType::Sqlite => Box::new(SqliteEngine),
        DBType::Duckdb => Box::new(DuckdbEngine),
    }
}

pub async fn run_query(
    daemon_registry: &daemon_registry::DaemonRegistry,
    path: &PathBuf,
    query: &str,
    db_type: &DBType,
    query_mode: QueryMode,
) -> Result<QueryResult, AybError> {
    let engine = engine_for(db_type);
    daemon_registry
        .execute_query(path, query, engine.db_type_str(), query_mode)
        .await
}
