pub mod paths;
mod sqlite;

use crate::ayb_db::models::DBType;
use crate::error::AybError;
use crate::hosted_db::sqlite::run_sqlite_query;
use crate::http::structs::AybConfigIsolation;
use prettytable::{format, Cell, Row, Table};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::vec::Vec;

#[derive(Serialize, Debug, Deserialize)]
pub struct QueryResult {
    pub fields: Vec<String>,
    pub rows: Vec<Vec<Option<String>>>,
}

impl QueryResult {
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

    pub fn generate_table(&self) -> Result<(), std::io::Error> {
        let mut table = self.to_table();
        table.set_format(*format::consts::FORMAT_NO_BORDER_LINE_SEPARATOR);
        table.print(&mut std::io::stdout())?;
        Ok(())
    }

    pub fn generate_csv(&self) -> Result<(), std::io::Error> {
        let table = self.to_table();
        table.to_csv(std::io::stdout())?;
        Ok(())
    }
}

// TODO(marcua): Consider a shared library so QueryResult can be the same in both crates, or move into the runner library.
impl From<ayb_hosted_db_runner::QueryResult> for QueryResult {
    fn from(results: ayb_hosted_db_runner::QueryResult) -> Self {
        QueryResult {
            fields: results.fields,
            rows: results.rows,
        }
    }
}

pub fn run_query(
    path: &PathBuf,
    query: &str,
    db_type: &DBType,
    isolation: &Option<AybConfigIsolation>,
) -> Result<QueryResult, AybError> {
    match db_type {
        DBType::Sqlite => Ok(run_sqlite_query(path, query, isolation)?),
        _ => Err(AybError {
            message: "Unsupported DB type".to_string(),
        }),
    }
}
