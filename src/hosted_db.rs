pub mod daemon_registry;
pub mod duckdb;
pub mod engine;
pub mod paths;
pub mod sandbox;
pub mod sqlite;

use crate::ayb_db::models::DBType;
// Used by the try_from_i16!/from_str! macro expansions below, which name
// AybError without it appearing textually in this file.
use crate::error::AybError;
use crate::formatting::TabularFormatter;
use crate::from_str;
use crate::hosted_db::duckdb::DuckdbEngine;
use crate::hosted_db::engine::DbEngine;
use crate::hosted_db::sqlite::SqliteEngine;
use crate::try_from_i16;
use prettytable::{Cell, Row, Table};
use serde::{Deserialize, Serialize};
use std::path::Path;
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

/// Render `path` as a single-quoted SQL string literal, doubling any
/// embedded single quotes.
///
/// Database paths contain entity and database slugs, which are
/// user-controlled, so they must never be interpolated into SQL raw.
/// Slugs are also validated at the API boundary (see
/// `server::slug_validation`); this is the second line of defense, at
/// the point where the string actually becomes SQL. It matters most for
/// snapshots, which build statements by interpolation and run them in
/// the server process rather than in a sandboxed daemon.
///
/// Doubling a single quote is the standard SQL escape and is understood
/// by both SQLite and DuckDB.
pub(crate) fn sql_string_literal(path: &Path) -> String {
    format!("'{}'", path.display().to_string().replace('\'', "''"))
}

pub fn engine_for(db_type: &DBType) -> &'static dyn DbEngine {
    match db_type {
        DBType::Sqlite => &SqliteEngine,
        DBType::Duckdb => &DuckdbEngine,
    }
}
