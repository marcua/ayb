use std::path::PathBuf;

use clap::{arg, command, value_parser, Command};

fn main() {
    let matches = command!()
        .subcommand(
            Command::new("query")
                .about("Query a DB")
                // TODO(marcua): Make this an enum.
                .arg(arg!(-t --type <VALUE> "The type of DB"))
                .arg(arg!(-q --query <VALUE> "The query to run"))
                .arg(arg!(-p --path <FILE> "Path to the DB").value_parser(value_parser!(PathBuf))),
        )
        .get_matches();

    if let Some(matches) = matches.subcommand_matches("query") {
        // Can I get rid of the let now that it's required?
        // Can I add an enum?
        // Can I make it run SQLite query?
        // Can I move the query subcommand into its own function?
        // Can I move the business logic into a library?
        if let (Some(path), Some(query), Some(db_type)) = (
            matches.get_one::<PathBuf>("path"),
            matches.get_one::<String>("query"),
            matches.get_one::<String>("type"),
        ) {
            println!("{} {} {}", path.display(), query, db_type);
        }
    }

    println!("Hello, world!");
}
