use assert_cmd::prelude::*;
use std::fs;
use std::process::Command;
use std::thread;
use std::time;

mod utils;

fn create_database(
    server_url: &str,
    api_key: &str,
    result: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    Command::cargo_bin("ayb")?
        .args([
            "client",
            "--url",
            server_url,
            "create_database",
            "e2e-first/test.sqlite",
            "sqlite",
        ])
        .env("AYB_API_TOKEN", api_key)
        .assert()
        .success()
        .stdout(format!("{}\n", result));
    Ok(())
}

fn query(
    server_url: &str,
    api_key: &str,
    query: &str,
    format: &str,
    result: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    Command::cargo_bin("ayb")?
        .args([
            "client",
            "--url",
            server_url,
            "query",
            "e2e-first/test.sqlite",
            "--format",
            format,
            query,
        ])
        .env("AYB_API_TOKEN", api_key)
        .assert()
        .success()
        .stdout(format!("{}\n", result));
    Ok(())
}

fn register(
    server_url: &str,
    slug: &str,
    email: &str,
    result: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    Command::cargo_bin("ayb")?
        .args(["client", "register", slug, email])
        .env("AYB_SERVER_URL", server_url)
        .assert()
        .success()
        .stdout(format!("{}\n", result));
    Ok(())
}

#[test]
fn client_server_integration_postgres() -> Result<(), Box<dyn std::error::Error>> {
    return client_server_integration("postgres", "http://127.0.0.1:5433", 10025);
}

#[test]
fn client_server_integration_sqlite() -> Result<(), Box<dyn std::error::Error>> {
    return client_server_integration("sqlite", "http://127.0.0.1:5434", 10026);
}

fn client_server_integration(
    db_type: &str,
    server_url: &str,
    smtp_port: u16,
) -> Result<(), Box<dyn std::error::Error>> {
    Command::new(format!("tests/reset_db_{}.sh", db_type))
        .assert()
        .success();

    // Run server
    let mut ayb_server = Command::cargo_bin("ayb")?
        .args([
            "server",
            "--config",
            &*format!("tests/test-server-config-{}.toml", db_type),
        ])
        .env("RUST_LOG", "actix_web=debug")
        .env("RUST_BACKTRACE", "1")
        .spawn()?;

    // Run stub SMTP server
    let mut smtp_server = Command::new("tests/smtp_server.sh")
        .args([&*format!("{}", smtp_port)])
        .spawn()?;

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
    Command::cargo_bin("ayb")?
        .args(["client", "confirm", &format!("{}a", first_token0)])
        .env("AYB_SERVER_URL", server_url)
        .assert()
        .success()
        .stdout("Error: Invalid or expired token\n");

    // Using either token first will register the account. The second
    // token, which has the same email address, will still work
    // (confirming an email the second time is like logging in). The
    // third token, which was with a different email address for the
    // same account, won't work now that there's already a confirmed
    // email address on the account..
    let first_api_key0 = utils::extract_api_key(
        Command::cargo_bin("ayb")?
            .args(["client", "confirm", &first_token0])
            .env("AYB_SERVER_URL", server_url)
            .assert()
            .success()
            .get_output(),
    )?;

    let first_api_key1 = utils::extract_api_key(
        Command::cargo_bin("ayb")?
            .args(["client", "confirm", &first_token1])
            .env("AYB_SERVER_URL", server_url)
            .assert()
            .success()
            .get_output(),
    )?;

    Command::cargo_bin("ayb")?
        .args(["client", "confirm", &first_token2])
        .env("AYB_SERVER_URL", server_url)
        .assert()
        .success()
        .stdout("Error: e2e-first has already been registered\n");

    // And for the second account, we can still confirm using the only
    // authentication token we've requested so far.
    let second_api_key0 = utils::extract_api_key(
        Command::cargo_bin("ayb")?
            .args(["client", "confirm", &second_token0])
            .env("AYB_SERVER_URL", server_url)
            .assert()
            .success()
            .get_output(),
    )?;

    // Logging in as the user emails the first email address, which
    // can confirm using the token it received.
    Command::cargo_bin("ayb")?
        .args(["client", "log_in", "e2e-first"])
        .env("AYB_SERVER_URL", server_url)
        .assert()
        .success()
        .stdout("Check your email to finish logging in e2e-first\n");

    let entries = utils::parse_smtp_log(&format!("tests/smtp_data_{}/e2e@example.org", smtp_port))?;
    assert_eq!(entries.len(), 3);
    let login_token = utils::extract_token(&entries[2])?;
    let first_api_key2 = utils::extract_api_key(
        Command::cargo_bin("ayb")?
            .args(["client", "confirm", &login_token])
            .env("AYB_SERVER_URL", server_url)
            .assert()
            .success()
            .get_output(),
    )?;

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

    // TODO(marcua): Make this cleanup code run even on test failure.
    // See https://medium.com/@ericdreichert/test-setup-and-teardown-in-rust-without-a-framework-ba32d97aa5ab
    if let Err(err) = fs::remove_dir_all("/tmp/ayb/e2e") {
        assert_eq!(format!("{}", err), "No such file or directory (os error 2)")
    }
    ayb_server.kill()?;
    smtp_server.kill()?;

    Ok(())
}
