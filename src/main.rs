use clap::{arg, command, value_parser, Command};
use stacks::hosted_db::run_query;
use stacks::http::client::StacksClient;
use stacks::http::server::run_server;
use stacks::stacks_db::models::DBType;
use std::path::PathBuf;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let matches = command!()
        .subcommand(
            Command::new("query")
                .about("Query a DB")
                .arg(arg!(--type <VALUE> "The type of DB").value_parser(value_parser!(DBType)))
                .arg(arg!(--query <VALUE> "The query to run"))
                .arg(arg!(--path <FILE> "Path to the DB").value_parser(value_parser!(PathBuf))),
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
        .subcommand(
            Command::new("client")
                .about("Connect to an HTTP server")
                .arg(
                    arg!(-p --port <VALUE> "The listener port")
                        .value_parser(value_parser!(u16))
                        .default_value("8000"),
                )
                .arg(arg!(--host <VALUE> "The host/IP to bind to").default_value("127.0.0.1"))
                .subcommand(
                    Command::new("create_database")
                        .about("Create a database")
                        .arg(arg!(--entity <VALUE> "The entity under which to create the DB"))
                        .arg(arg!(--database <VALUE> "The DBto create"))
                        .arg(
                            arg!(--type <VALUE> "The type of DB")
                                .value_parser(value_parser!(DBType)),
                        ),
                ),
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
            run_server(host, port).await?;
        }
    } else if let Some(matches) = matches.subcommand_matches("client") {
        if let (Some(host), Some(port)) = (
            matches.get_one::<String>("host"),
            matches.get_one::<u16>("port"),
        ) {
            let base_url = format!("http://{}:{}", host, port);
            let client = StacksClient { base_url };
            if let Some(matches) = matches.subcommand_matches("create_database") {
                if let (Some(entity), Some(database), Some(db_type)) = (
                    matches.get_one::<String>("entity"),
                    matches.get_one::<String>("database"),
                    matches.get_one::<DBType>("type"),
                ) {
                    match client.create_database(entity, database, db_type).await {
                        Ok(response) => {
                            println!("Response is: {}", response);
                        }
                        Err(err) => {
                            println!("Error is: {}", err);
                        }
                    }
                }
            }
        }
    }

    Ok(())
}
