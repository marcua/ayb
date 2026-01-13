#![allow(clippy::too_many_arguments)]

mod browser_e2e_tests;
mod e2e_tests;
mod utils;

use crate::browser_e2e_tests::{
    test_create_and_query_database_flow, test_entity_profile_flow, test_permissions_flow,
    test_registration_flow, test_snapshots_flow,
};
use crate::e2e_tests::{
    test_create_and_query_db, test_entity_details_and_profile, test_health_check, test_permissions,
    test_registration, test_snapshots,
};
use crate::utils::browser::BrowserHelpers;
use crate::utils::email::clear_email_data;
use crate::utils::testing::{
    ensure_minio_running, get_test_port, reset_test_environment, AybServer, Cleanup,
};
use assert_cmd::prelude::*;
use ayb::client::config::ClientConfig;
use regex::Regex;
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
    test_type: &str,
    server_url: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let config_path = format!("tests/ayb_data_{test_type}/ayb.json");
    let mut expected_config = ClientConfig::new();
    let _cleanup = Cleanup;

    // Ensure MinIO is running
    ensure_minio_running()?;

    reset_test_environment(test_type)?;

    // Run server
    let _ayb_server = AybServer::run(test_type).expect("failed to start the ayb server");

    // Give the external processes time to start
    thread::sleep(time::Duration::from_secs(10));

    // Test health endpoint first (doesn't require authentication)
    test_health_check(server_url).await?;

    let api_keys = test_registration(test_type, &config_path, server_url, &mut expected_config)?;
    test_create_and_query_db(&config_path, &api_keys, server_url, &mut expected_config)?;
    test_entity_details_and_profile(&config_path, &api_keys)?;
    test_snapshots(test_type, &config_path, &api_keys).await?;
    test_permissions(&config_path, &api_keys).await?;

    Ok(())
}

#[tokio::test]
async fn browser_e2e() -> Result<(), Box<dyn std::error::Error>> {
    let _cleanup = Cleanup;

    // Ensure MinIO is running
    ensure_minio_running()?;

    // Reset database
    reset_test_environment("browser_sqlite")?;

    // Clear email data for browser test
    clear_email_data("browser_sqlite")?;

    // Start ayb server
    let _ayb_server = AybServer::run("browser_sqlite").expect("failed to start the ayb server");

    // Give servers time to start
    tokio::time::sleep(std::time::Duration::from_secs(5)).await;

    // Initialize browser using helper method
    let (_playwright, contexts_and_pages) = BrowserHelpers::set_up_browser(1).await?;
    let (_context, page) = &contexts_and_pages[0];

    // Construct base URL using the port from test configuration
    let port = get_test_port("browser_sqlite")?;
    let base_url = format!("http://127.0.0.1:{}", port);

    // Run registration test and get the username
    let username = test_registration_flow(&page, &base_url, "browser_sqlite").await?;

    // Continue with profile test using the registered user
    test_entity_profile_flow(&page, &username).await?;

    // Continue with database creation and query test
    test_create_and_query_database_flow(&page, &username).await?;

    // Test multi-user permissions with separate browser contexts
    test_permissions_flow(&base_url, "browser_sqlite").await?;

    // Test snapshots functionality
    test_snapshots_flow(&page, &username, &base_url).await?;

    Ok(())
}
