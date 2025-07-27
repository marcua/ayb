#![allow(clippy::too_many_arguments)]

mod browser_e2e_tests;
mod e2e_tests;
mod email_helpers;
mod utils;

use crate::browser_e2e_tests::test_registration_flow;
use crate::e2e_tests::{
    test_create_and_query_db, test_entity_details_and_profile, test_permissions, test_registration,
    test_snapshots,
};
use crate::utils::testing::{AybServer, Cleanup};
use assert_cmd::prelude::*;
use ayb::client::config::ClientConfig;
use regex::Regex;
use std::process::Command;
use std::thread;
use std::time;

#[tokio::test]
async fn client_server_integration_postgres() -> Result<(), Box<dyn std::error::Error>> {
    client_server_integration("postgres", "http://127.0.0.1:5433").await
}

#[tokio::test]
async fn client_server_integration_sqlite() -> Result<(), Box<dyn std::error::Error>> {
    client_server_integration("sqlite", "http://127.0.0.1:5434").await
}

#[test]
fn default_server_config() -> Result<(), Box<dyn std::error::Error>> {
    let re = Regex::new(r#"fernet_key = "[^"]+""#).unwrap();
    let expected = r#"host = "0.0.0.0"
port = 5433
database_url = "sqlite://ayb_data/ayb.sqlite"
data_path = "./ayb_data"

[authentication]
!!!fernet_line!!!
token_expiration_seconds = 3600

[email.smtp]
from = "Server Sender <server@example.org>"
reply_to = "Server Reply <replyto@example.org>"
smtp_host = "localhost"
smtp_port = 465
smtp_username = "login@example.org"
smtp_password = "the_password"

[email.file]
path = "./ayb_data/emails.jsonl"

[web]
hosting_method = "Local"

[cors]
origin = "*"

"#;
    let cmd = ayb_assert_cmd!("default_server_config"; {});
    let output = std::str::from_utf8(&cmd.get_output().stdout)?;
    assert_eq!(re.replace_all(output, "!!!fernet_line!!!"), expected);
    Ok(())
}

async fn client_server_integration(
    db_type: &str,
    server_url: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let config_path = format!("tests/ayb_data_{db_type}/ayb.json");
    let mut expected_config = ClientConfig::new();
    let _cleanup = Cleanup;

    Command::new(format!("tests/reset_db_{db_type}.sh"))
        .assert()
        .success();

    // Run server
    let _ayb_server = AybServer::run(db_type).expect("failed to start the ayb server");

    // Give the external processes time to start
    thread::sleep(time::Duration::from_secs(10));

    let api_keys = test_registration(&config_path, server_url, &mut expected_config)?;
    test_create_and_query_db(&config_path, &api_keys, server_url, &mut expected_config)?;
    test_entity_details_and_profile(&config_path, &api_keys)?;
    test_snapshots(db_type, &config_path, &api_keys).await?;
    test_permissions(&config_path, &api_keys).await?;

    Ok(())
}

#[tokio::test]
async fn browser_e2e() -> Result<(), Box<dyn std::error::Error>> {
    use crate::utils::browser::BrowserHelpers;

    let _cleanup = Cleanup;

    // Reset database
    std::process::Command::new("tests/reset_db_sqlite.sh")
        .output()
        .expect("Failed to reset database");

    // Start ayb server
    let _ayb_server = AybServer::run("sqlite").expect("failed to start the ayb server");

    // Give servers time to start
    tokio::time::sleep(std::time::Duration::from_secs(5)).await;

    // Initialize browser using helper method
    let (_playwright, page) = BrowserHelpers::setup_browser().await?;

    test_registration_flow(&page).await
}
