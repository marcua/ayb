use assert_cmd::prelude::*;
use std::fs;
use std::process::Command;
use std::thread;
use std::time;

fn client_query(query: &str, format: &str, result: &str) -> Result<(), Box<dyn std::error::Error>> {
    Command::cargo_bin("stacks")?
        .args([
            "client",
            "--url",
            "http://127.0.0.1:5433",
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
fn client_server_integration() -> Result<(), Box<dyn std::error::Error>> {
    Command::new("tests/reset_db.sh").assert().success();

    // Run server, give it a few seconds to start
    let mut server = Command::cargo_bin("stacks")?
        .args(["server", "--config", "tests/test-server-config.toml"])
        .spawn()?;
    thread::sleep(time::Duration::from_secs(1));

    // Register an entity.
    Command::cargo_bin("stacks")?
        .args(["client", "register", "e2e"])
        .env("STACKS_SERVER_URL", "http://127.0.0.1:5433")
        .assert()
        .success()
        .stdout("Successfully registered e2e\n");

    // Can't register an entity twice.
    Command::cargo_bin("stacks")?
        .args(["client", "register", "e2e"])
        .env("STACKS_SERVER_URL", "http://127.0.0.1:5433")
        .assert()
        .success()
        .stdout("Error: Entity already exists\n");

    // Create database.
    Command::cargo_bin("stacks")?
        .args([
            "client",
            "--url",
            "http://127.0.0.1:5433",
            "create_database",
            "e2e/test.sqlite",
            "sqlite",
        ])
        .assert()
        .success()
        .stdout("Successfully created e2e/test.sqlite\n");

    // Can't create a database twice.
    Command::cargo_bin("stacks")?
        .args([
            "client",
            "--url",
            "http://127.0.0.1:5433",
            "create_database",
            "e2e/test.sqlite",
            "sqlite",
        ])
        .assert()
        .success()
        .stdout("Error: Database already exists\n");

    // Populate and query database.
    client_query(
        "CREATE TABLE test_table(fname varchar, lname varchar);",
        "table",
        "\nRows: 0",
    )?;
    client_query(
        "INSERT INTO test_table (fname, lname) VALUES (\"the first\", \"the last\");",
        "table",
        "\nRows: 0",
    )?;
    client_query(
        "INSERT INTO test_table (fname, lname) VALUES (\"the first2\", \"the last2\");",
        "table",
        "\nRows: 0",
    )?;
    client_query("SELECT * FROM test_table;",
                 "table",                 
                 " fname      | lname \n------------+-----------\n the first  | the last \n the first2 | the last2 \n\nRows: 2")?;
    client_query(
        "SELECT * FROM test_table;",
        "csv",
        "fname,lname\nthe first,the last\nthe first2,the last2\n\nRows: 2",
    )?;

    // TODO(marcua): Make this cleanup code run even on test failure.
    // See https://medium.com/@ericdreichert/test-setup-and-teardown-in-rust-without-a-framework-ba32d97aa5ab
    if let Err(err) = fs::remove_dir_all("/tmp/stacks/e2e") {
        assert_eq!(format!("{}", err), "No such file or directory (os error 2)")
    }
    server.kill()?;

    Ok(())
}
