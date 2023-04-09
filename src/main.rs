use clap::builder::ValueParser;
use clap::{arg, command, value_parser, Command, ValueEnum};
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

#[derive(Clone, ValueEnum)]
pub enum OutputFormat {
    Table = 0,
    Csv = 1,
}

impl OutputFormat {
    pub fn to_str(&self) -> &str {
        match self {
            OutputFormat::Table => "table",
            OutputFormat::Csv => "csv",
        }
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
                .arg(arg!(--config <FILE> "Path to the server's configuration file")
                     .value_parser(value_parser!(PathBuf))
                     .env("STACKS_SERVER_CONFIG_FILE")
                     .default_value("./stacks.toml"))
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
                    Command::new("query")
                        .about("Query a database")
                        .arg(arg!(<database> "The database to which to connect (e.g., entity/database.sqlite")
                             .value_parser(ValueParser::new(entity_database_parser))
                             .required(true)
                        )
                        .arg(arg!(<query> "The query to execute")
                             .required(true))
                        .arg(
                            arg!(--format <type> "The format in which to output the result")
                                .value_parser(value_parser!(OutputFormat))
                                .default_value(OutputFormat::Table.to_str())
                                .required(false)),
                )
                .subcommand(
                    Command::new("register")
                        .about("Register a user/organization")
                        .arg(arg!(<entity> "The entity to create")
                             .required(true))
                        .arg(
                            arg!(<type> "The type of entity")
                                .value_parser(value_parser!(EntityType))
                                .default_value(EntityType::User.to_str())
                                .required(false)),
                )
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
        if let Some(config) = matches.get_one::<PathBuf>("config") {
            run_server(config).await?;
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
            } else if let Some(matches) = matches.subcommand_matches("register") {
                if let (Some(entity), Some(entity_type)) = (
                    matches.get_one::<String>("entity"),
                    matches.get_one::<EntityType>("type"),
                ) {
                    match client.register(entity, entity_type).await {
                        Ok(_response) => {
                            println!("Successfully registered {}", entity);
                        }
                        Err(err) => {
                            println!("Error: {}", err);
                        }
                    }
                }
            } else if let Some(matches) = matches.subcommand_matches("query") {
                if let (Some(entity_database), Some(query), Some(format)) = (
                    matches.get_one::<EntityDatabasePath>("database"),
                    matches.get_one::<String>("query"),
                    matches.get_one::<OutputFormat>("format"),
                ) {
                    match client
                        .query(&entity_database.entity, &entity_database.database, query)
                        .await
                    {
                        Ok(query_result) => {
                            if query_result.rows.len() > 0 {
                                match format {
                                    OutputFormat::Table => query_result.generate_table()?,
                                    OutputFormat::Csv => query_result.generate_csv()?,
                                }
                            }
                            println!("\nRows: {}", query_result.rows.len());
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
