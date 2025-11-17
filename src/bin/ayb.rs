use ayb::client::cli::{client_commands, execute_client_command};
use ayb::server::cli::{execute_server_command, server_commands};
use ayb::server::config::{config_to_toml, default_server_config};
use clap::{command, Command};

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let matches = command!()
        .subcommand(server_commands())
        .subcommand(
            Command::new("default_server_config")
                .about("Print a default configuration file for a server"),
        )
        .subcommand(client_commands())
        .get_matches();

    if let Some(matches) = matches.subcommand_matches("server") {
        // Check if a subcommand was provided
        let has_subcommand = matches.subcommand().is_some();
        execute_server_command(matches, !has_subcommand).await?;
    } else if let Some(_matches) = matches.subcommand_matches("default_server_config") {
        match config_to_toml(default_server_config()) {
            Ok(config) => println!("{config}"),
            Err(err) => println!("Error: {err}"),
        }
    } else if let Some(matches) = matches.subcommand_matches("client") {
        execute_client_command(matches).await?;
    }

    Ok(())
}
