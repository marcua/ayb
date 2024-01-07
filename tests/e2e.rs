#![allow(clippy::too_many_arguments)]

use assert_cmd::prelude::*;
use ayb::client::config::ClientConfig;
use regex::Regex;
use std::fs;
use std::process::{Child, Command};
use std::thread;
use std::time;

mod utils;

// ayb_cmd!("value1", value2; {
//     "ENV_VAR" => env_value
// })
macro_rules! ayb_cmd {
    ($($value:expr),+; { $($env_left:literal => $env_right:expr),* $(,)? }) => {
        Command::cargo_bin("ayb")?
                .args([$($value,)*])
                $(.env($env_left, $env_right))*
    }
}

// ayb_assert_cmd!("value1", value2; {
//     "ENV_VAR" => env_value
// })
macro_rules! ayb_assert_cmd {
    ($($value:expr),+; { $($env_left:literal => $env_right:expr),* $(,)? }) => {
        Command::cargo_bin("ayb")?
                .args([$($value,)*])
                $(.env($env_left, $env_right))*
                .assert()
                .success()
    }
}

struct Cleanup;

impl Drop for Cleanup {
    fn drop(&mut self) {
        if let Err(err) = fs::remove_dir_all("/tmp/ayb/e2e") {
            assert_eq!(format!("{}", err), "No such file or directory (os error 2)")
        }
    }
}

struct AybServer(Child);
impl AybServer {
    fn run(db_type: &str) -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self(
            ayb_cmd!("server", "--config", &format!("tests/test-server-config-{}.toml", db_type); {
                "RUST_LOG" => "actix_web=debug",
                "RUST_BACKTRACE" => "1"
            })
            .spawn()?,
        ))
    }
}

impl Drop for AybServer {
    fn drop(&mut self) {
        let _ = self.0.kill();
    }
}

struct SmtpServer(Child);

impl SmtpServer {
    fn run(smtp_port: u16) -> Result<Self, Box<dyn std::error::Error>> {
        Ok(SmtpServer(
            Command::new("tests/smtp_server.sh")
                .args([&*format!("{}", smtp_port)])
                .spawn()?,
        ))
    }
}

impl Drop for SmtpServer {
    fn drop(&mut self) {
        let _ = self.0.kill();
    }
}

fn create_database(
    config: &str,
    api_key: &str,
    result: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let cmd = ayb_assert_cmd!("client", "--config", config, "create_database", "e2e-first/test.sqlite", "sqlite"; {
        "AYB_API_TOKEN" => api_key,
    });

    cmd.stdout(format!("{}\n", result));
    Ok(())
}

fn query(
    config: &str,
    api_key: &str,
    query: &str,
    format: &str,
    result: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let cmd = ayb_assert_cmd!("client", "--config", config, "query", "e2e-first/test.sqlite", "--format", format, query; {
        "AYB_API_TOKEN" => api_key,
    });

    cmd.stdout(format!("{}\n", result));
    Ok(())
}

fn query_no_api_token(
    config: &str,
    query: &str,
    format: &str,
    result: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let cmd = ayb_assert_cmd!("client", "--config", config, "query", "e2e-first/test.sqlite", "--format", format, query; {});

    cmd.stdout(format!("{}\n", result));
    Ok(())
}

fn set_default_url(
    config: &str,
    server_url: &str,
    result: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let cmd = ayb_assert_cmd!("client", "--config", config, "set_default_url", server_url; {});

    cmd.stdout(format!("{}\n", result));
    Ok(())
}

fn register(
    config: &str,
    server_url: &str,
    slug: &str,
    email: &str,
    result: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let cmd = ayb_assert_cmd!("client", "register", slug, email; {
        "AYB_CLIENT_CONFIG_FILE" => config,
        "AYB_SERVER_URL" => server_url,
    });

    cmd.stdout(format!("{}\n", result));
    Ok(())
}

fn list_databases(
    config: &str,
    api_key: &str,
    entity: &str,
    format: &str,
    result: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let cmd = ayb_assert_cmd!("client", "--config", config, "list", entity, "--format", format; {
        "AYB_API_TOKEN" => api_key,
    });

    cmd.stdout(format!("{}\n", result));
    Ok(())
}

fn profile(
    config: &str,
    api_key: &str,
    entity: &str,
    format: &str,
    result: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let cmd = ayb_assert_cmd!("client", "--config", config, "profile", entity, "--format", format; {
        "AYB_API_TOKEN" => api_key,
    });

    cmd.stdout(format!("{}\n", result));
    Ok(())
}

fn update_profile(
    config: &str,
    api_key: &str,
    entity: &str,
    display_name: Option<&str>,
    description: Option<&str>,
    organization: Option<&str>,
    location: Option<&str>,
    links: Option<Vec<&str>>,
    result: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = Command::cargo_bin("ayb")?;
    cmd.args(["client", "--config", config, "update_profile", entity])
        .env("AYB_API_TOKEN", api_key);

    if let Some(display_name) = display_name {
        cmd.arg("--display_name").arg(display_name);
    }

    if let Some(description) = description {
        cmd.arg("--description").arg(description);
    }

    if let Some(organization) = organization {
        cmd.arg("--organization").arg(organization);
    }

    if let Some(location) = location {
        cmd.arg("--location").arg(location);
    }

    if let Some(links) = links {
        cmd.arg("--links").arg(links.join(","));
    }

    cmd.assert().success().stdout(format!("{}\n", result));
    Ok(())
}

#[test]
fn client_server_integration_postgres() -> Result<(), Box<dyn std::error::Error>> {
    client_server_integration("postgres", "http://127.0.0.1:5433", 10025)
}

#[test]
fn client_server_integration_sqlite() -> Result<(), Box<dyn std::error::Error>> {
    client_server_integration("sqlite", "http://127.0.0.1:5434", 10026)
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

[email]
from = "Server Sender <server@example.org>"
reply_to = "Server Reply <replyto@example.org>"
smtp_host = "localhost"
smtp_port = 465
smtp_username = "login@example.org"
smtp_password = "the_password"

[cors]
origin = "*"

"#;
    let cmd = ayb_assert_cmd!("default_server_config"; {});
    let output = std::str::from_utf8(&cmd.get_output().stdout)?;
    assert_eq!(re.replace_all(output, "!!!fernet_line!!!"), expected);
    Ok(())
}

fn client_server_integration(
    db_type: &str,
    server_url: &str,
    smtp_port: u16,
) -> Result<(), Box<dyn std::error::Error>> {
    let config_path = format!("tests/ayb_data_{}/ayb.json", db_type);
    let mut expected_config = ClientConfig::new();
    let _cleanup = Cleanup;

    Command::new(format!("tests/reset_db_{}.sh", db_type))
        .assert()
        .success();

    // Run server
    let _ayb_server = AybServer::run(db_type).expect("failed to start the ayb server");

    // Run stub SMTP server
    let _smtp_server = SmtpServer::run(smtp_port).expect("failed to start the smtp server");

    // Give the external processes time to start
    thread::sleep(time::Duration::from_secs(10));

    let first_entity_0 = "e2e-first";

    // Before running commands, we have no configuration file
    assert_eq!(
        fs::read_to_string(&config_path).unwrap_err().kind(),
        std::io::ErrorKind::NotFound
    );

    // Register an entity.
    register(
        &config_path,
        server_url,
        first_entity_0,
        "e2e@example.org",
        "Check your email to finish registering e2e-first",
    )?;

    // The configuration file should register the server URL.
    expected_config.default_url = Some(server_url.to_string());
    assert_eq!(
        fs::read_to_string(&config_path).unwrap(),
        serde_json::to_string(&expected_config)?
    );

    // Register the same entity with the same email address.
    register(
        &config_path,
        server_url,
        first_entity_0,
        "e2e@example.org",
        "Check your email to finish registering e2e-first",
    )?;

    // Can start to register an entity twice with different email
    // addresses as long as you don't complete the process.
    register(
        &config_path,
        server_url,
        first_entity_0,
        "e2e-another@example.org",
        "Check your email to finish registering e2e-first",
    )?;

    let second_entity_0 = "e2e-second";

    // Start the registration process for a second user (e2e-second)
    register(
        &config_path,
        server_url,
        second_entity_0,
        "e2e-another@example.org",
        "Check your email to finish registering e2e-second",
    )?;

    // Check that two emails were received
    let entries = utils::parse_smtp_log(&format!("tests/smtp_data_{}/e2e@example.org", smtp_port))?;
    assert_eq!(entries.len(), 2);
    let first_token0 = utils::extract_token(&entries[0])?;
    let first_token1 = utils::extract_token(&entries[1])?;
    let entries = utils::parse_smtp_log(&format!(
        "tests/smtp_data_{}/e2e-another@example.org",
        smtp_port
    ))?;
    assert_eq!(entries.len(), 2);
    let first_token2 = utils::extract_token(&entries[0])?;
    let second_token0 = utils::extract_token(&entries[1])?;

    // Using a bad token (appending a letter) doesn't work.
    let cmd = ayb_assert_cmd!("client", "confirm", &format!("{}a", first_token0); {
        "AYB_CLIENT_CONFIG_FILE" => config_path.clone(),
    });
    cmd.stdout("Error: Invalid or expired token\n");
    assert_eq!(
        fs::read_to_string(&config_path).unwrap(),
        serde_json::to_string(&expected_config)?
    );

    // Using either token first will register the account. The second
    // token, which has the same email address, will still work
    // (confirming an email the second time is like logging in). The
    // third token, which was with a different email address for the
    // same account, won't work now that there's already a confirmed
    // email address on the account..
    let cmd = ayb_assert_cmd!("client", "confirm", &first_token0; {
        "AYB_CLIENT_CONFIG_FILE" => config_path.clone(),
    });
    let first_api_key0 = utils::extract_api_key(cmd.get_output())?;
    expected_config
        .authentication
        .insert(server_url.to_string(), first_api_key0.clone());
    assert_eq!(
        fs::read_to_string(&config_path).unwrap(),
        serde_json::to_string(&expected_config)?
    );

    let cmd = ayb_assert_cmd!("client", "confirm", &first_token1; {
        "AYB_CLIENT_CONFIG_FILE" => config_path.clone(),
    });
    let first_api_key1 = utils::extract_api_key(cmd.get_output())?;
    expected_config
        .authentication
        .insert(server_url.to_string(), first_api_key1.clone());
    assert_eq!(
        fs::read_to_string(&config_path).unwrap(),
        serde_json::to_string(&expected_config)?
    );

    let cmd = ayb_assert_cmd!("client", "confirm", &first_token2; {
        "AYB_CLIENT_CONFIG_FILE" => config_path.clone(),
    });
    cmd.stdout("Error: e2e-first has already been registered\n");
    assert_eq!(
        fs::read_to_string(&config_path).unwrap(),
        serde_json::to_string(&expected_config)?
    );

    // And for the second account, we can still confirm using the only
    // authentication token we've requested so far.
    let cmd = ayb_assert_cmd!("client", "confirm", &second_token0; {
        "AYB_CLIENT_CONFIG_FILE" => config_path.clone(),
    });
    let second_api_key0 = utils::extract_api_key(cmd.get_output())?;
    expected_config
        .authentication
        .insert(server_url.to_string(), second_api_key0.clone());
    assert_eq!(
        fs::read_to_string(&config_path).unwrap(),
        serde_json::to_string(&expected_config)?
    );

    // Logging in as the user emails the first email address, which
    // can confirm using the token it received.
    let cmd = ayb_assert_cmd!("client", "log_in", "e2e-first"; {
        "AYB_CLIENT_CONFIG_FILE" => config_path.clone(),
    });

    cmd.stdout("Check your email to finish logging in e2e-first\n");

    let entries = utils::parse_smtp_log(&format!("tests/smtp_data_{}/e2e@example.org", smtp_port))?;
    assert_eq!(entries.len(), 3);
    let login_token = utils::extract_token(&entries[2])?;

    let cmd = ayb_assert_cmd!("client", "confirm", &login_token; {
        "AYB_CLIENT_CONFIG_FILE" => config_path.clone(),
    });
    let first_api_key2 = utils::extract_api_key(cmd.get_output())?;
    expected_config
        .authentication
        .insert(server_url.to_string(), first_api_key2.clone());
    assert_eq!(
        fs::read_to_string(&config_path).unwrap(),
        serde_json::to_string(&expected_config)?
    );
    // To summarize where we are at this point
    // * User e2e-first has three API tokens (first_api_key[0...2]). We'll use these
    //   interchangeably below.
    // * User e2e-second has one API token (second_api_key0)

    // Can't create database on e2e-first with e2e-second's token.
    create_database(
        &config_path,
        &second_api_key0,
        "Error: Authenticated entity e2e-second can not create a database for entity e2e-first",
    )?;

    // Can't create database on e2e-first with invalid token.
    create_database(
        &config_path,
        &format!("{}bad", first_api_key0),
        "Error: Invalid API token",
    )?;

    // Create a database with the appropriate user/key pair.
    create_database(
        &config_path,
        &first_api_key0,
        "Successfully created e2e-first/test.sqlite",
    )?;

    // Can't create a database twice.
    create_database(
        &config_path,
        &first_api_key0,
        "Error: Database already exists",
    )?;

    // Can't query database with second account's API key
    query(
        &config_path,
        &second_api_key0,
        "CREATE TABLE test_table(fname varchar, lname varchar);",
        "table",
        "Error: Authenticated entity e2e-second can not query database e2e-first/test.sqlite",
    )?;

    // Can't query database with bad API key.
    query(
        &config_path,
        &format!("{}bad", first_api_key0),
        "CREATE TABLE test_table(fname varchar, lname varchar);",
        "table",
        "Error: Invalid API token",
    )?;

    // Populate and query database. Alternate through the three API
    // keys for the first account to ensure they all work.
    query(
        &config_path,
        &first_api_key0,
        "CREATE TABLE test_table(fname varchar, lname varchar);",
        "table",
        "\nRows: 0",
    )?;
    query(
        &config_path,
        &first_api_key1,
        "INSERT INTO test_table (fname, lname) VALUES (\"the first\", \"the last\");",
        "table",
        "\nRows: 0",
    )?;
    query(
        &config_path,
        &first_api_key2,
        "INSERT INTO test_table (fname, lname) VALUES (\"the first2\", \"the last2\");",
        "table",
        "\nRows: 0",
    )?;
    query(
        &config_path,
        &first_api_key0,
        "SELECT * FROM test_table;",
                 "table",                 
                 " fname      | lname \n------------+-----------\n the first  | the last \n the first2 | the last2 \n\nRows: 2")?;
    query(
        &config_path,
        &first_api_key0,
        "SELECT * FROM test_table;",
        "csv",
        "fname,lname\nthe first,the last\nthe first2,the last2\n\nRows: 2",
    )?;

    // Querying with no API token also works, because the first
    // account is saved in the configuration file.
    query_no_api_token(
        &config_path,
        "SELECT * FROM test_table;",
        "csv",
        "fname,lname\nthe first,the last\nthe first2,the last2\n\nRows: 2",
    )?;

    // Testing setting the default server URL: we set it to something
    // nonsensical and it breaks connections.
    set_default_url(
        &config_path,
        &format!("{}badport", server_url),
        &format!("Saved {}badport as new default_url", server_url),
    )?;
    expected_config.default_url = Some(format!("{}badport", server_url));
    assert_eq!(
        fs::read_to_string(&config_path).unwrap(),
        serde_json::to_string(&expected_config)?
    );
    query(
        &config_path,
        &first_api_key0,
        "SELECT * FROM test_table;",
        "csv",
        "Error: reqwest::Error { kind: Builder, source: InvalidPort }",
    )?;
    set_default_url(
        &config_path,
        server_url,
        &format!("Saved {} as new default_url", server_url),
    )?;
    expected_config.default_url = Some(server_url.to_string());
    assert_eq!(
        fs::read_to_string(&config_path).unwrap(),
        serde_json::to_string(&expected_config)?
    );
    query(
        &config_path,
        &first_api_key0,
        "SELECT * FROM test_table;",
        "csv",
        "fname,lname\nthe first,the last\nthe first2,the last2\n\nRows: 2",
    )?;

    // List databases from first account using its API key
    list_databases(
        &config_path,
        &first_api_key0,
        first_entity_0,
        "csv",
        "Database slug,Type\ntest.sqlite,sqlite",
    )?;

    // List databases from first account using the API key of the second account
    list_databases(
        &config_path,
        &second_api_key0,
        first_entity_0,
        "csv",
        &format!("No queryable databases owned by {}", first_entity_0),
    )?;

    // Make some partial profile updates and verify profile details upon retrieval
    update_profile(
        &config_path,
        &first_api_key0,
        first_entity_0,
        Some("Entity 0"),
        Some("Entity 0 description"),
        None,
        None,
        None,
        "Successfully updated profile",
    )?;

    profile(
        &config_path,
        &first_api_key0,
        first_entity_0,
        "csv",
        "Display name,Description,Organization,Location,Links\nEntity 0,Entity 0 description,null,null,"
    )?;

    profile(
        &config_path,
        &second_api_key0,
        first_entity_0,
        "csv",
        "Display name,Description,Organization,Location,Links\nEntity 0,Entity 0 description,null,null,"
    )?;

    update_profile(
        &config_path,
        &first_api_key0,
        first_entity_0,
        None,
        Some("Entity 0 NEW description"),
        Some("Entity 0 organization"),
        None,
        Some(vec!["http://ayb.host/", "http://ayb2.host"]),
        "Successfully updated profile",
    )?;

    profile(
        &config_path,
        &first_api_key0,
        first_entity_0,
        "csv",
        "Display name,Description,Organization,Location,Links\nEntity 0,Entity 0 NEW description,Entity 0 organization,null,\"http://ayb.host/,http://ayb2.host\""
    )?;

    profile(
        &config_path,
        &second_api_key0,
        first_entity_0,
        "csv",
        "Display name,Description,Organization,Location,Links\nEntity 0,Entity 0 NEW description,Entity 0 organization,null,\"http://ayb.host/,http://ayb2.host\""
    )?;

    Ok(())
}
