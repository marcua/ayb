pub mod paths;
mod sandbox;
pub mod sqlite;

use crate::ayb_db::models::{DBType, InstantiatedDatabase};
use crate::error::AybError;
use crate::formatting::TabularFormatter;
use crate::hosted_db::paths::database_path;
use crate::hosted_db::sqlite::potentially_isolated_sqlite_query;
use crate::server::config::AybConfigIsolation;
use prettytable::{Cell, Row, Table};
use serde::{Deserialize, Serialize};
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

/// `allow_unsafe` disables features that prevent abuse but also
/// prevent backups/snapshots. The only known use case in the codebase
/// is for snapshots.
pub async fn run_query(
    entity_slug: &str,
    database: &InstantiatedDatabase,
    query: &str,
    data_path: &str,
    isolation: &Option<AybConfigIsolation>,
    allow_unsafe: bool,
) -> Result<QueryResult, AybError> {
    if isolation.is_some() && allow_unsafe {
        return Err(AybError::InvalidIsolationConfiguration {
            message: "Can't run isolated query in unsafe mode".to_string(),
        });
    }
    let path = database_path(entity_slug, &database.slug, data_path, false)?;
    match DBType::try_from(database.db_type)? {
        DBType::Sqlite => {
            Ok(potentially_isolated_sqlite_query(&path, query, isolation, allow_unsafe).await?)
        }
        _ => Err(AybError::Other {
            message: "Unsupported DB type".to_string(),
        }),
    }
}
