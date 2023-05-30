use assert_cmd::prelude::*;
use std::fs;
use std::process::Command;
use std::thread;
use std::time;

fn client_query(query: &str, format: &str, result: &str) -> Result<(), Box<dyn std::error::Error>> {
    Command::cargo_bin("ayb")?
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
fn client_server_integration_postgres() -> Result<(), Box<dyn std::error::Error>> {
    return client_server_integration("postgres");
}

#[test]
fn client_server_integration_sqlite() -> Result<(), Box<dyn std::error::Error>> {
    return client_server_integration("sqlite");
}

fn client_server_integration(db_type: &str) -> Result<(), Box<dyn std::error::Error>> {
    Command::new(format!("tests/reset_db_{}.sh", db_type)).assert().success();

    // Run server, give it a few seconds to start
    let mut server = Command::cargo_bin("ayb")?
        .args(["server", "--config", &*format!("tests/test-server-config-{}.toml", db_type)])
        .spawn()?;
    thread::sleep(time::Duration::from_secs(1));

    // Register an entity.
    Command::cargo_bin("ayb")?
        .args(["client", "register", "e2e"])
        .env("AYB_SERVER_URL", "http://127.0.0.1:5433")
        .assert()
        .success()
        .stdout("Successfully registered e2e\n");

    // Can't register an entity twice.
    Command::cargo_bin("ayb")?
        .args(["client", "register", "e2e"])
        .env("AYB_SERVER_URL", "http://127.0.0.1:5433")
        .assert()
        .success()
        .stdout("Error: Entity already exists\n");

    // Create database.
    Command::cargo_bin("ayb")?
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
    Command::cargo_bin("ayb")?
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
    if let Err(err) = fs::remove_dir_all("/tmp/ayb/e2e") {
        assert_eq!(format!("{}", err), "No such file or directory (os error 2)")
    }
    server.kill()?;

    Ok(())
}
