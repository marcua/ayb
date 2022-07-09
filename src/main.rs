use std::fmt;
use std::path::PathBuf;

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

fn run_sqlite_query(path: &PathBuf, query: &str) -> rusqlite::Result<()> {
    let conn = rusqlite::Connection::open(path)?;
    conn.execute(query, [])?;
    Ok(())
}

fn run_query(path: &PathBuf, query: &str, db_type: &DBType) -> Result<(), &'static str> {
    match db_type {
        DBType::Sqlite => {
            match run_sqlite_query(path, query) {
                Ok(_result) => println!("Got result"),
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
        // Results -> Hashmap/JSON and print
        // Move business logic into library
        // Wrap in HTTP API
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
