use ayb::client::cli::{client_commands, execute_client_command};
use ayb::server::config::{config_to_toml, default_server_config};
use ayb::server::server_runner::run_server;
use clap::{arg, command, value_parser, Command};
use std::path::PathBuf;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let matches = command!()
        .subcommand(
            Command::new("server").about("Run an HTTP server").arg(
                arg!(--config <FILE> "Path to the server's configuration file")
                    .value_parser(value_parser!(PathBuf))
                    .env("AYB_SERVER_CONFIG_FILE")
                    .default_value("./ayb.toml"),
            ),
        )
        .subcommand(
            Command::new("default_server_config")
                .about("Print a default configuration file for a server"),
        )
        .subcommand(client_commands())
        .get_matches();

    if let Some(matches) = matches.subcommand_matches("server") {
        if let Some(config) = matches.get_one::<PathBuf>("config") {
            run_server(config).await?;
        }
    } else if let Some(_matches) = matches.subcommand_matches("default_server_config") {
        match config_to_toml(default_server_config()) {
            Ok(config) => println!("{}", config),
            Err(err) => println!("Error: {}", err),
        }
    } else if let Some(matches) = matches.subcommand_matches("client") {
        execute_client_command(matches).await?;
    }

    Ok(())
}
