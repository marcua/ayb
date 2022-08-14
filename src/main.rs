mod databases;

use clap::{arg, command, value_parser, Command};
use databases::{run_query, DBType};
use std::path::PathBuf;

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
