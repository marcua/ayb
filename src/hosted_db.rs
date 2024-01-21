pub mod paths;
mod sandbox;
pub mod sqlite;

use crate::ayb_db::models::DBType;
use crate::error::AybError;
use crate::formatting::TabularFormatter;
use crate::hosted_db::sqlite::potentially_isolated_sqlite_query;
use crate::server::config::AybConfigIsolation;
use prettytable::{Cell, Row, Table};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::vec::Vec;

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

pub async fn run_query(
    path: &PathBuf,
    query: &str,
    db_type: &DBType,
    isolation: &Option<AybConfigIsolation>,
) -> Result<QueryResult, AybError> {
    match db_type {
        DBType::Sqlite => Ok(potentially_isolated_sqlite_query(path, query, isolation).await?),
        _ => Err(AybError::Other {
            message: "Unsupported DB type".to_string(),
        }),
    }
}
