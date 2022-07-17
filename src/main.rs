use std::collections::HashMap;
use std::fmt;
use std::path::PathBuf;
use std::vec::Vec;

use clap::{arg, command, value_parser, Command, ValueEnum};
use rusqlite;

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
enum DBType {
    Sqlite,
    Duckdb,
}

impl fmt::Display for DBType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

fn run_sqlite_query(
    path: &PathBuf,
    query: &str,
) -> Result<Vec<HashMap<String, String>>, rusqlite::Error> {
    let conn = rusqlite::Connection::open(path)?;
    let mut prepared = conn.prepare(query)?;
    let num_columns = prepared.column_count();
    let mut column_names = HashMap::new();
    for column_index in 0..num_columns {
        column_names.insert(
            column_index,
            String::from(prepared.column_name(column_index)?),
        );
    }

    let mut rows = prepared.query([])?;
    let mut results: Vec<HashMap<String, String>> = Vec::new();
    while let Some(row) = rows.next()? {
        let mut row_map: HashMap<String, String> = HashMap::new();
        for column_index in 0..num_columns {
            row_map.insert(
                column_names.get(&column_index).unwrap().to_string(),
                row.get(column_index)?,
            );
        }
        results.push(row_map);
    }
    Ok(results)
}

fn run_query(path: &PathBuf, query: &str, db_type: &DBType) -> Result<(), &'static str> {
    match db_type {
        DBType::Sqlite => {
            match run_sqlite_query(path, query) {
                Ok(result) => {
                    println!("The result: {:#?}", result)
                }
                Err(err) => {
                    println!("SQLite error: {}", err);
                }
            }
            Ok(())
        }
        _ => Err("Unsupported DB type"),
    }
}

fn main() -> Result<(), &'static str> {
    let matches = command!()
        .subcommand(
            Command::new("query")
                .about("Query a DB")
                .arg(arg!(-t --type <VALUE> "The type of DB").value_parser(value_parser!(DBType)))
                .arg(arg!(-q --query <VALUE> "The query to run"))
                .arg(arg!(-p --path <FILE> "Path to the DB").value_parser(value_parser!(PathBuf))),
        )
        .get_matches();

    if let Some(matches) = matches.subcommand_matches("query") {
        if let (Some(path), Some(query), Some(db_type)) = (
            matches.get_one::<PathBuf>("path"),
            matches.get_one::<String>("query"),
            matches.get_one::<DBType>("type"),
        ) {
            run_query(path, &query, db_type)?;
        }
    }

    Ok(())
}
