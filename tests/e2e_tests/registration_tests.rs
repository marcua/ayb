use crate::ayb_assert_cmd;
use crate::e2e_tests::{FIRST_ENTITY_SLUG, SECOND_ENTITY_SLUG, THIRD_ENTITY_SLUG};
use crate::utils::ayb::register;
use assert_cmd::prelude::*;
use ayb::client::config::ClientConfig;
use ayb::error::AybError;
use regex::Regex;
use serde::{Deserialize, Serialize};
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

#[derive(Serialize, Deserialize)]
struct EmailEntry {
    from: String,
    to: String,
    reply_to: String,
    subject: String,
    content_type: String,
    content_transfer_encoding: String,
    date: String,
    content: Vec<String>,
}

fn extract_token(email: &EmailEntry) -> Result<String, AybError> {
    let prefix = "\tayb client confirm ";
    assert_eq!(email.subject, "Your login credentials");
    for line in &email.content {
        if line.starts_with(prefix) && line.len() > prefix.len() {
            return Ok(String::from_utf8(quoted_printable::decode(
                &line[prefix.len()..],
                quoted_printable::ParseMode::Robust,
            )?)?);
        }
    }
    Err(AybError::Other {
        message: "No token found in email".to_string(),
    })
}

fn parse_smtp_log(file_path: &str) -> Result<Vec<EmailEntry>, serde_json::Error> {
    let mut entries = Vec::new();
    for line in fs::read_to_string(file_path).unwrap().lines() {
        entries.push(serde_json::from_str(line)?);
    }
    Ok(entries)
}

pub fn test_registration(
    config_path: &str,
    server_url: &str,
    smtp_port: u16,
    expected_config: &mut ClientConfig,
) -> Result<HashMap<String, Vec<String>>, Box<dyn std::error::Error>> {
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

    // Check that two emails were received
    let entries = parse_smtp_log(&format!("tests/smtp_data_{}/e2e@example.org", smtp_port))?;
    assert_eq!(entries.len(), 2);
    let first_token0 = extract_token(&entries[0])?;
    let first_token1 = extract_token(&entries[1])?;
    let entries = parse_smtp_log(&format!(
        "tests/smtp_data_{}/e2e-another@example.org",
        smtp_port
    ))?;
    assert_eq!(entries.len(), 2);
    let first_token2 = extract_token(&entries[0])?;
    let second_token0 = extract_token(&entries[1])?;

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

    let entries = parse_smtp_log(&format!("tests/smtp_data_{}/e2e@example.org", smtp_port))?;
    assert_eq!(entries.len(), 3);
    let login_token = extract_token(&entries[2])?;

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
    let entries = parse_smtp_log(&format!(
        "tests/smtp_data_{}/e2e-a-third@example.org",
        smtp_port
    ))?;
    assert_eq!(entries.len(), 1);
    let third_token0 = extract_token(&entries[0])?;
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

pub fn test_banned_username_registration(
    config_path: &str,
    server_url: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // Test that banned usernames are rejected during registration

    // Test ayb-specific route conflicts
    let ayb_banned = ["register", "log_in", "log_out", "confirm", "v1"];
    for banned_username in ayb_banned {
        let cmd = ayb_assert_cmd!("client", "register", banned_username, "test@example.org"; {
            "AYB_CLIENT_CONFIG_FILE" => config_path,
            "AYB_SERVER_URL" => server_url,
        });
        cmd.stdout(format!(
            "Error: Username '{}' is reserved and cannot be used\n",
            banned_username
        ));
    }

    // Test common reserved names from shouldbee/reserved-usernames
    let common_banned = ["admin", "root", "www", "api", "support", "help"];
    for banned_username in common_banned {
        let cmd = ayb_assert_cmd!("client", "register", banned_username, "test@example.org"; {
            "AYB_CLIENT_CONFIG_FILE" => config_path,
            "AYB_SERVER_URL" => server_url,
        });
        cmd.stdout(format!(
            "Error: Username '{}' is reserved and cannot be used\n",
            banned_username
        ));
    }

    // Test additional comprehensive reserved names
    let extended_banned = ["blog", "news", "email", "contact", "about", "null"];
    for banned_username in extended_banned {
        let cmd = ayb_assert_cmd!("client", "register", banned_username, "test@example.org"; {
            "AYB_CLIENT_CONFIG_FILE" => config_path,
            "AYB_SERVER_URL" => server_url,
        });
        cmd.stdout(format!(
            "Error: Username '{}' is reserved and cannot be used\n",
            banned_username
        ));
    }

    // Test that case doesn't matter - all should be banned
    let case_banned = ["REGISTER", "Log_In", "API", "ROOT"];
    for banned_username in case_banned {
        let cmd = ayb_assert_cmd!("client", "register", banned_username, "test@example.org"; {
            "AYB_CLIENT_CONFIG_FILE" => config_path,
            "AYB_SERVER_URL" => server_url,
        });
        cmd.stdout(format!(
            "Error: Username '{}' is reserved and cannot be used\n",
            banned_username.to_lowercase()
        ));
    }

    // Test that valid usernames still work
    let cmd = ayb_assert_cmd!("client", "register", "validusername", "test@example.org"; {
        "AYB_CLIENT_CONFIG_FILE" => config_path,
        "AYB_SERVER_URL" => server_url,
    });
    cmd.stdout("Check your email to finish registering validusername\n");

    Ok(())
}
