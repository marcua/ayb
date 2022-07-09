use std::fmt;
use std::path::PathBuf;

use clap::{arg, command, value_parser, Command, ValueEnum};

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

fn main() {
    let matches = command!()
        .subcommand(
            Command::new("query")
                .about("Query a DB")
                .arg(arg!(-t --type <VALUE> "The type of DB").value_parser(value_parser!(DBType)))
                .arg(arg!(-q --query <VALUE> "The query to run"))
                .arg(arg!(-p --path <FILE> "Path to the DB").value_parser(value_parser!(PathBuf)))
        )
        .get_matches();

    if let Some(matches) = matches.subcommand_matches("query") {
        // Can I make it run SQLite query?
        // Can I move the query subcommand into its own function?
        // Can I move the business logic into a library?
        if let (Some(path), Some(query), Some(db_type)) = (
            matches.get_one::<PathBuf>("path"),
            matches.get_one::<String>("query"),
            matches.get_one::<DBType>("type"),
        ) {
            println!("{} {} {}", path.display(), query, db_type);
        }
    }

    println!("Hello, world!");
}
