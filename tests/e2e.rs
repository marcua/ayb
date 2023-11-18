use assert_cmd::prelude::*;
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
        let child =
            ayb_cmd!("server", "--config", &format!("tests/test-server-config-{}.toml", db_type); {
                "RUST_LOG" => "actix_web=debug",
                "RUST_BACKTRACE" => "1"
            })
            .spawn();

        Ok(Self(child?))
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
    server_url: &str,
    api_key: &str,
    result: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let cmd = ayb_assert_cmd!("client", "--url", server_url, "create_database", "e2e-first/test.sqlite", "sqlite"; {
        "AYB_API_TOKEN" => api_key,
    });

    cmd.stdout(format!("{}\n", result));
    Ok(())
}

fn query(
    server_url: &str,
    api_key: &str,
    query: &str,
    format: &str,
    result: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let cmd = ayb_assert_cmd!("client", "--url", server_url, "query", "e2e-first/test.sqlite", "--format", format, query; {
        "AYB_API_TOKEN" => api_key,
    });

    cmd.stdout(format!("{}\n", result));
    Ok(())
}

fn register(
    server_url: &str,
    slug: &str,
    email: &str,
    result: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let cmd = ayb_assert_cmd!("client", "register", slug, email; {
        "AYB_SERVER_URL" => server_url,
    });

    cmd.stdout(format!("{}\n", result));
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

    // Register an entity.
    register(
        server_url,
        "e2e-first",
        "e2e@example.org",
        "Check your email to finish registering e2e-first",
    )?;

    // Register the same entity with the same email address.
    register(
        server_url,
        "e2e-first",
        "e2e@example.org",
        "Check your email to finish registering e2e-first",
    )?;

    // Can start to register an entity twice with different email
    // addresses as long as you don't complete the process.
    register(
        server_url,
        "e2e-first",
        "e2e-another@example.org",
        "Check your email to finish registering e2e-first",
    )?;

    // Start the registration process for a second user (e2e-second)
    register(
        server_url,
        "e2e-second",
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
        "AYB_SERVER_URL" => server_url,
    });
    cmd.stdout("Error: Invalid or expired token\n");

    // Using either token first will register the account. The second
    // token, which has the same email address, will still work
    // (confirming an email the second time is like logging in). The
    // third token, which was with a different email address for the
    // same account, won't work now that there's already a confirmed
    // email address on the account..
    let cmd = ayb_assert_cmd!("client", "confirm", &first_token0; {
        "AYB_SERVER_URL" => server_url,
    });
    let first_api_key0 = utils::extract_api_key(cmd.get_output())?;

    let cmd = ayb_assert_cmd!("client", "confirm", &first_token1; {
        "AYB_SERVER_URL" => server_url,
    });
    let first_api_key1 = utils::extract_api_key(cmd.get_output())?;

    let cmd = ayb_assert_cmd!("client", "confirm", &first_token2; {
        "AYB_SERVER_URL" => server_url,
    });
    cmd.stdout("Error: e2e-first has already been registered\n");

    // And for the second account, we can still confirm using the only
    // authentication token we've requested so far.
    let cmd = ayb_assert_cmd!("client", "confirm", &second_token0; {
        "AYB_SERVER_URL" => server_url,
    });
    let second_api_key0 = utils::extract_api_key(cmd.get_output())?;

    // Logging in as the user emails the first email address, which
    // can confirm using the token it received.
    let cmd = ayb_assert_cmd!("client", "log_in", "e2e-first"; {
        "AYB_SERVER_URL" => server_url,
    });

    cmd.stdout("Check your email to finish logging in e2e-first\n");

    let entries = utils::parse_smtp_log(&format!("tests/smtp_data_{}/e2e@example.org", smtp_port))?;
    assert_eq!(entries.len(), 3);
    let login_token = utils::extract_token(&entries[2])?;

    let cmd = ayb_assert_cmd!("client", "confirm", &login_token; {
        "AYB_SERVER_URL" => server_url,
    });
    let first_api_key2 = utils::extract_api_key(cmd.get_output())?;

    // To summarize where we are at this point
    // * User e2e-first has three API tokens (first_api_key[0...2]). We'll use these
    //   interchangeably below.
    // * User e2e-second has one API token (second_api_key0)

    // Can't create database on e2e-first with e2e-second's token.
    create_database(
        server_url,
        &second_api_key0,
        "Error: Authenticated entity e2e-second can not create a database for entity e2e-first",
    )?;

    // Can't create database on e2e-first with invalid token.
    create_database(
        server_url,
        &format!("{}bad", first_api_key0),
        "Error: Invalid API token",
    )?;

    // Create a database with the appropriate user/key pair.
    create_database(
        server_url,
        &first_api_key0,
        "Successfully created e2e-first/test.sqlite",
    )?;

    // Can't create a database twice.
    create_database(
        server_url,
        &first_api_key0,
        "Error: Database already exists",
    )?;

    // Can't query database with second account's API key
    query(
        server_url,
        &second_api_key0,
        "CREATE TABLE test_table(fname varchar, lname varchar);",
        "table",
        "Error: Authenticated entity e2e-second can not query database e2e-first/test.sqlite",
    )?;

    // Can't query database with bad API key.
    query(
        server_url,
        &format!("{}bad", first_api_key0),
        "CREATE TABLE test_table(fname varchar, lname varchar);",
        "table",
        "Error: Invalid API token",
    )?;

    // Populate and query database. Alternate through the three API
    // keys for the first account to ensure they all work.
    query(
        server_url,
        &first_api_key0,
        "CREATE TABLE test_table(fname varchar, lname varchar);",
        "table",
        "\nRows: 0",
    )?;
    query(
        server_url,
        &first_api_key1,
        "INSERT INTO test_table (fname, lname) VALUES (\"the first\", \"the last\");",
        "table",
        "\nRows: 0",
    )?;
    query(
        server_url,
        &first_api_key2,
        "INSERT INTO test_table (fname, lname) VALUES (\"the first2\", \"the last2\");",
        "table",
        "\nRows: 0",
    )?;
    query(
        server_url,
        &first_api_key0,
        "SELECT * FROM test_table;",
                 "table",                 
                 " fname      | lname \n------------+-----------\n the first  | the last \n the first2 | the last2 \n\nRows: 2")?;
    query(
        server_url,
        &first_api_key0,
        "SELECT * FROM test_table;",
        "csv",
        "fname,lname\nthe first,the last\nthe first2,the last2\n\nRows: 2",
    )?;

    Ok(())
}
