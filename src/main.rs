use clap::builder::ValueParser;
use clap::{arg, command, value_parser, Command};
use regex::Regex;
use stacks::hosted_db::run_query;
use stacks::http::client::StacksClient;
use stacks::http::server::run_server;
use stacks::http::structs::EntityDatabasePath;
use stacks::stacks_db::models::{DBType, EntityType};
use std::path::PathBuf;

fn entity_database_parser(value: &str) -> Result<EntityDatabasePath, String> {
    let re = Regex::new(r"^(\S+)/(\S+)$").unwrap();
    if re.is_match(value) {
        let captures = re.captures(value).unwrap();
        Ok(EntityDatabasePath {
            entity: captures.get(1).map_or("", |m| m.as_str()).to_string(),
            database: captures.get(2).map_or("", |m| m.as_str()).to_string(),
        })
    } else {
        Err("Argument must be formatted as 'entity/database'".to_string())
    }
}

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
                    arg!(--url <VALUE> "The server URL")
                        .env("STACKS_SERVER_URL")
                        .required(true)
                )
                .subcommand(
                    Command::new("create_database")
                        .about("Create a database")
                        .arg(arg!(<database> "The database to create (e.g., entity/database.sqlite")
                             .value_parser(ValueParser::new(entity_database_parser))
                             .required(true)
                        )
                        .arg(
                            arg!(<type> "The type of DB")
                                .value_parser(value_parser!(DBType))
                                .default_value(DBType::Sqlite.to_str())
                                .required(false)
                        ),
                )
                .subcommand(
                    Command::new("create_entity")
                        .about("Create an entity")
                        .arg(arg!(<entity> "The entity to create")
                             .required(true))
                        .arg(
                            arg!(<type> "The type of entity")
                                .value_parser(value_parser!(EntityType))
                                .default_value(EntityType::User.to_str())
                                .required(false)),
                )
                .subcommand(
                    Command::new("query")
                        .about("Query a database")
                        .arg(arg!(<database> "The database to which to connect (e.g., entity/database.sqlite")
                             .value_parser(ValueParser::new(entity_database_parser))
                             .required(true)
                        )
                        .arg(arg!(<query> "The query to execute")
                             .required(true)),
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
        if let Some(url) = matches.get_one::<String>("url") {
            let client = StacksClient {
                base_url: url.to_string(),
            };
            if let Some(matches) = matches.subcommand_matches("create_database") {
                if let (Some(entity_database), Some(db_type)) = (
                    matches.get_one::<EntityDatabasePath>("database"),
                    matches.get_one::<DBType>("type"),
                ) {
                    match client
                        .create_database(
                            &entity_database.entity,
                            &entity_database.database,
                            db_type,
                        )
                        .await
                    {
                        Ok(_response) => {
                            println!(
                                "Successfully created {}/{}",
                                entity_database.entity, entity_database.database
                            );
                        }
                        Err(err) => {
                            println!("Error: {}", err);
                        }
                    }
                }
            } else if let Some(matches) = matches.subcommand_matches("create_entity") {
                if let (Some(entity), Some(entity_type)) = (
                    matches.get_one::<String>("entity"),
                    matches.get_one::<EntityType>("type"),
                ) {
                    match client.create_entity(entity, entity_type).await {
                        Ok(_response) => {
                            println!("Successfully registered {}", entity);
                        }
                        Err(err) => {
                            println!("Error: {}", err);
                        }
                    }
                }
            } else if let Some(matches) = matches.subcommand_matches("query") {
                if let (Some(entity_database), Some(query)) = (
                    matches.get_one::<EntityDatabasePath>("database"),
                    matches.get_one::<String>("query"),
                ) {
                    match client
                        .query(&entity_database.entity, &entity_database.database, query)
                        .await
                    {
                        Ok(response) => {
                            println!("Response is: {:?}", response);
                        }
                        Err(err) => {
                            println!("Error: {}", err);
                        }
                    }
                }
            }
        }
    }

    Ok(())
}
