use assert_cmd::prelude::*;
use std::fs;
use std::process::Command;
use std::thread;
use std::time;

mod utils;

fn client_query(
    server_url: &str,
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
            "e2e/test.sqlite",
            "--format",
            format,
            query,
        ])
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

    // Run server, give it a few seconds to start
    let mut ayb_server = Command::cargo_bin("ayb")?
        .args([
            "server",
            "--config",
            &*format!("tests/test-server-config-{}.toml", db_type),
        ])
        .spawn()?;
    thread::sleep(time::Duration::from_secs(1));

    // Run stub SMTP server, give it a few seconds to start
    let mut smtp_server = Command::new("tests/smtp_server.sh")
        .args([&*format!("{}", smtp_port)])
        .spawn()?;
    thread::sleep(time::Duration::from_secs(20));

    // Register an entity.
    Command::cargo_bin("ayb")?
        .args(["client", "register", "e2e", "e2e@example.org"])
        .env("AYB_SERVER_URL", server_url)
        .assert()
        .success()
        .stdout("Check your email to finish registering e2e\n");

    // Register the same entity with the same email address.
    Command::cargo_bin("ayb")?
        .args(["client", "register", "e2e", "e2e@example.org"])
        .env("AYB_SERVER_URL", server_url)
        .assert()
        .success()
        .stdout("Check your email to finish registering e2e\n");

    // Can start to register an entity twice as long as you don't
    // complete the process.
    Command::cargo_bin("ayb")?
        .args(["client", "register", "e2e", "e2e-another@example.org"])
        .env("AYB_SERVER_URL", server_url)
        .assert()
        .success()
        .stdout("Check your email to finish registering e2e\n");

    // Check that two emails were received
    let entries = utils::parse_smtp_log(&format!("tests/smtp_data_{}/e2e@example.org", smtp_port))?;
    assert_eq!(entries.len(), 2);
    let token0 = utils::extract_token(&entries[0])?;
    let token1 = utils::extract_token(&entries[1])?;
    let entries = utils::parse_smtp_log(&format!(
        "tests/smtp_data_{}/e2e-another@example.org",
        smtp_port
    ))?;
    assert_eq!(entries.len(), 1);
    let token2 = utils::extract_token(&entries[0])?;

    // Using a bad token (appending a letter) doesn't work.
    Command::cargo_bin("ayb")?
        .args(["client", "confirm", &format!("{}a", token0)])
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
    Command::cargo_bin("ayb")?
        .args(["client", "confirm", &token0])
        .env("AYB_SERVER_URL", server_url)
        .assert()
        .success()
        .stdout("Successfully authenticated and saved token default/insecure, unimplemented\n");

    Command::cargo_bin("ayb")?
        .args(["client", "confirm", &token1])
        .env("AYB_SERVER_URL", server_url)
        .assert()
        .success()
        .stdout("Successfully authenticated and saved token default/insecure, unimplemented\n");

    Command::cargo_bin("ayb")?
        .args(["client", "confirm", &token2])
        .env("AYB_SERVER_URL", server_url)
        .assert()
        .success()
        .stdout("Error: e2e has already been registered\n");

    // Logging in as the user emails the first email address, which
    // can confirm using the token it received.
    Command::cargo_bin("ayb")?
        .args(["client", "log_in", "e2e"])
        .env("AYB_SERVER_URL", server_url)
        .assert()
        .success()
        .stdout("Check your email to finish logging in e2e\n");

    let entries = utils::parse_smtp_log(&format!("tests/smtp_data_{}/e2e@example.org", smtp_port))?;
    assert_eq!(entries.len(), 3);
    let login_token = utils::extract_token(&entries[2])?;
    Command::cargo_bin("ayb")?
        .args(["client", "confirm", &login_token])
        .env("AYB_SERVER_URL", server_url)
        .assert()
        .success()
        .stdout("Successfully authenticated and saved token default/insecure, unimplemented\n");

    // Create database.
    Command::cargo_bin("ayb")?
        .args([
            "client",
            "--url",
            server_url,
            "create_database",
            "e2e/test.sqlite",
            "sqlite",
        ])
        .assert()
        .success()
        .stdout("Successfully created e2e/test.sqlite\n");

    // Can't create a database twice.
    Command::cargo_bin("ayb")?
        .args([
            "client",
            "--url",
            server_url,
            "create_database",
            "e2e/test.sqlite",
            "sqlite",
        ])
        .assert()
        .success()
        .stdout("Error: Database already exists\n");

    // Populate and query database.
    client_query(
        server_url,
        "CREATE TABLE test_table(fname varchar, lname varchar);",
        "table",
        "\nRows: 0",
    )?;
    client_query(
        server_url,
        "INSERT INTO test_table (fname, lname) VALUES (\"the first\", \"the last\");",
        "table",
        "\nRows: 0",
    )?;
    client_query(
        server_url,
        "INSERT INTO test_table (fname, lname) VALUES (\"the first2\", \"the last2\");",
        "table",
        "\nRows: 0",
    )?;
    client_query(
        server_url,
        "SELECT * FROM test_table;",
                 "table",                 
                 " fname      | lname \n------------+-----------\n the first  | the last \n the first2 | the last2 \n\nRows: 2")?;
    client_query(
        server_url,
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
