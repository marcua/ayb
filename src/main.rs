use clap::{arg, command, value_parser, Command};
use stacks::hosted_db::run_query;
use stacks::http::run_server;
use stacks::stacks_db::models::DBType;
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
        .subcommand(
            Command::new("server")
                .about("Run an HTTP server")
                .arg(
                    arg!(-p --port <VALUE> "The listener port")
                        .value_parser(value_parser!(u16))
                        .default_value("8000"),
                )
                .arg(arg!(--host <VALUE> "The host/IP to bind to").default_value("127.0.0.1")),
        )
        .get_matches();

    if let Some(matches) = matches.subcommand_matches("query") {
        if let (Some(path), Some(query), Some(db_type)) = (
            matches.get_one::<PathBuf>("path"),
            matches.get_one::<String>("query"),
            matches.get_one::<DBType>("type"),
        ) {
            match run_query(path, &query, db_type) {
                Ok(result) => {
                    println!("Result schema: {:#?}", result.fields);
                    println!("Results: {:#?}", result.rows);
                }
                Err(err) => {
                    println!("{}", err);
                }
            }
        }
    } else if let Some(matches) = matches.subcommand_matches("server") {
        if let (Some(host), Some(port)) = (
            matches.get_one::<String>("host"),
            matches.get_one::<u16>("port"),
        ) {
            match run_server(host, port) {
                Ok(_result) => {
                    println!("Server is stopping...")
                }
                Err(err) => {
                    println!("Unable to run server {}", err);
                }
            }
        }
    }

    Ok(())
}
