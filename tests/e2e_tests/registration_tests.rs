use crate::ayb_assert_cmd;
use crate::e2e_tests::{FIRST_ENTITY_SLUG, SECOND_ENTITY_SLUG, THIRD_ENTITY_SLUG};
use crate::email_helpers::{clear_email_file, extract_token_from_emails, parse_email_file};
use crate::utils::ayb::register;
use assert_cmd::prelude::*;
use ayb::client::config::ClientConfig;
use ayb::error::AybError;
use regex::Regex;
use std::collections::HashMap;
use std::fs;
use std::process::{Command, Output};

fn extract_api_key(output: &Output) -> Result<String, AybError> {
    let output_str = std::str::from_utf8(&output.stdout)?;
    let re = Regex::new(r"^Successfully authenticated (\S+) and saved token (\S+)\n").unwrap();
    if re.is_match(output_str) {
        let captures = re.captures(output_str).unwrap();
        Ok(captures.get(2).map_or("", |m| m.as_str()).to_string())
    } else {
        Err(AybError::Other {
            message: "No API key".to_string(),
        })
    }
}

const SQLITE_EMAIL_FILE: &str = "tests/ayb_data_sqlite/emails.jsonl";
const POSTGRES_EMAIL_FILE: &str = "tests/ayb_data_postgres/emails.jsonl";

fn get_email_file_for_test(config_path: &str) -> &'static str {
    if config_path.contains("sqlite") {
        SQLITE_EMAIL_FILE
    } else {
        POSTGRES_EMAIL_FILE
    }
}

pub fn clear_email_data(config_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let email_file = get_email_file_for_test(config_path);
    clear_email_file(email_file)?;
    Ok(())
}

fn get_emails_for_recipient(
    config_path: &str,
    recipient: &str,
) -> Result<Vec<ayb::email::backend::EmailEntry>, Box<dyn std::error::Error>> {
    let email_file = get_email_file_for_test(config_path);
    let emails = parse_email_file(email_file)?;

    let filtered_emails = emails
        .into_iter()
        .filter(|email| email.to == recipient)
        .collect();

    Ok(filtered_emails)
}

pub fn test_registration(
    config_path: &str,
    server_url: &str,
    expected_config: &mut ClientConfig,
) -> Result<HashMap<String, Vec<String>>, Box<dyn std::error::Error>> {
    // Clear any existing email data
    clear_email_data(config_path)?;

    // Before running commands, we have no configuration file
    assert_eq!(
        fs::read_to_string(config_path).unwrap_err().kind(),
        std::io::ErrorKind::NotFound
    );

    // Register an entity.
    register(
        config_path,
        server_url,
        FIRST_ENTITY_SLUG,
        "e2e@example.org",
        "Check your email to finish registering e2e-first",
    )?;

    // The configuration file should register the server URL.
    expected_config.default_url = Some(server_url.to_string());
    assert_eq!(
        fs::read_to_string(config_path).unwrap(),
        serde_json::to_string(&expected_config)?
    );

    // Register the same entity with the same email address.
    register(
        config_path,
        server_url,
        FIRST_ENTITY_SLUG,
        "e2e@example.org",
        "Check your email to finish registering e2e-first",
    )?;

    // Can start to register an entity twice with different email
    // addresses as long as you don't complete the process.
    register(
        config_path,
        server_url,
        FIRST_ENTITY_SLUG,
        "e2e-another@example.org",
        "Check your email to finish registering e2e-first",
    )?;

    // Start the registration process for a second user (e2e-second)
    register(
        config_path,
        server_url,
        SECOND_ENTITY_SLUG,
        "e2e-another@example.org",
        "Check your email to finish registering e2e-second",
    )?;

    // Get emails for each recipient
    let e2e_emails = get_emails_for_recipient(config_path, "e2e@example.org")?;
    assert_eq!(e2e_emails.len(), 2);
    let first_token0 = extract_token_from_emails(&[e2e_emails[0].clone()]).unwrap();
    let first_token1 = extract_token_from_emails(&[e2e_emails[1].clone()]).unwrap();

    let another_emails = get_emails_for_recipient(config_path, "e2e-another@example.org")?;
    assert_eq!(another_emails.len(), 2);
    let first_token2 = extract_token_from_emails(&[another_emails[0].clone()]).unwrap();
    let second_token0 = extract_token_from_emails(&[another_emails[1].clone()]).unwrap();

    // Using a bad token (appending a letter) doesn't work.
    let cmd = ayb_assert_cmd!("client", "confirm", &format!("{}a", first_token0); {
        "AYB_CLIENT_CONFIG_FILE" => config_path,
    });
    cmd.stdout("Error: Invalid or expired token\n");
    assert_eq!(
        fs::read_to_string(config_path).unwrap(),
        serde_json::to_string(&expected_config)?
    );

    // Using either token first will register the account. The second
    // token, which has the same email address, will still work
    // (confirming an email the second time is like logging in). The
    // third token, which was with a different email address for the
    // same account, won't work now that there's already a confirmed
    // email address on the account..
    let cmd = ayb_assert_cmd!("client", "confirm", &first_token0; {
        "AYB_CLIENT_CONFIG_FILE" => config_path,
    });
    let first_api_key0 = extract_api_key(cmd.get_output())?;
    expected_config
        .authentication
        .insert(server_url.to_string(), first_api_key0.clone());
    assert_eq!(
        fs::read_to_string(config_path).unwrap(),
        serde_json::to_string(&expected_config)?
    );

    let cmd = ayb_assert_cmd!("client", "confirm", &first_token1; {
        "AYB_CLIENT_CONFIG_FILE" => config_path,
    });
    let first_api_key1 = extract_api_key(cmd.get_output())?;
    expected_config
        .authentication
        .insert(server_url.to_string(), first_api_key1.clone());
    assert_eq!(
        fs::read_to_string(config_path).unwrap(),
        serde_json::to_string(&expected_config)?
    );

    let cmd = ayb_assert_cmd!("client", "confirm", &first_token2; {
        "AYB_CLIENT_CONFIG_FILE" => config_path,
    });
    cmd.stdout("Error: e2e-first has already been registered\n");
    assert_eq!(
        fs::read_to_string(config_path).unwrap(),
        serde_json::to_string(&expected_config)?
    );

    // And for the second account, we can still confirm using the only
    // authentication token we've requested so far.
    let cmd = ayb_assert_cmd!("client", "confirm", &second_token0; {
        "AYB_CLIENT_CONFIG_FILE" => config_path,
    });
    let second_api_key0 = extract_api_key(cmd.get_output())?;
    expected_config
        .authentication
        .insert(server_url.to_string(), second_api_key0.clone());
    assert_eq!(
        fs::read_to_string(config_path).unwrap(),
        serde_json::to_string(&expected_config)?
    );

    // Logging in as the user emails the first email address, which
    // can confirm using the token it received.
    let cmd = ayb_assert_cmd!("client", "log_in", "e2e-first"; {
        "AYB_CLIENT_CONFIG_FILE" => config_path,
    });

    cmd.stdout("Check your email to finish logging in e2e-first\n");

    let e2e_emails = get_emails_for_recipient(config_path, "e2e@example.org")?;
    assert_eq!(e2e_emails.len(), 3);
    let login_token = extract_token_from_emails(&[e2e_emails[2].clone()]).unwrap();

    let cmd = ayb_assert_cmd!("client", "confirm", &login_token; {
        "AYB_CLIENT_CONFIG_FILE" => config_path,
    });
    let first_api_key2 = extract_api_key(cmd.get_output())?;
    expected_config
        .authentication
        .insert(server_url.to_string(), first_api_key2.clone());
    assert_eq!(
        fs::read_to_string(config_path).unwrap(),
        serde_json::to_string(&expected_config)?
    );

    // Start the registration process for a third user (e2e-third)
    register(
        config_path,
        server_url,
        THIRD_ENTITY_SLUG,
        "e2e-a-third@example.org",
        "Check your email to finish registering e2e-third",
    )?;

    let third_emails = get_emails_for_recipient(config_path, "e2e-a-third@example.org")?;
    assert_eq!(third_emails.len(), 1);
    let third_token0 = extract_token_from_emails(&[third_emails[0].clone()]).unwrap();

    let cmd = ayb_assert_cmd!("client", "confirm", &third_token0; {
        "AYB_CLIENT_CONFIG_FILE" => format!("{}-throwaway", config_path), // Don't save this third account's credentials as our default token in the main configuration file.
        "AYB_SERVER_URL" => server_url,
    });
    let third_api_key0 = extract_api_key(cmd.get_output())?;

    // To summarize where we are at this point
    // * User e2e-first has three API tokens (first_api_key[0...2]). We'll use these
    //   interchangeably in subsequent tests.
    // * User e2e-second has one API token (second_api_key0)
    // * User e2e-third has one API token (third_api_key0)
    let mut api_keys: HashMap<String, Vec<String>> = HashMap::new();
    api_keys.insert(
        "first".to_string(),
        vec![first_api_key0, first_api_key1, first_api_key2],
    );
    api_keys.insert("second".to_string(), vec![second_api_key0]);
    api_keys.insert("third".to_string(), vec![third_api_key0]);
    Ok(api_keys)
}
