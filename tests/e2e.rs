use assert_cmd::prelude::*;
use std::fs;
use std::process::Command;
use std::thread;
use std::time;

fn client_query(query: &str, result: &str) -> Result<(), Box<dyn std::error::Error>> {
    Command::cargo_bin("stacks")?
        .args([
            "client",
            "--url",
            "http://127.0.0.1:8000",
            "query",
            "e2e/test.sqlite",
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
        .args(["server", "--host", "127.0.0.1", "--port", "8000"])
        .spawn()?;
    thread::sleep(time::Duration::from_secs(1));

    // Create entity.
    Command::cargo_bin("stacks")?
        .args(["client", "create_entity", "e2e"])
        .env("STACKS_SERVER_URL", "http://127.0.0.1:8000")
        .assert()
        .success()
        .stdout("Response is: InstantiatedEntity { id: 1, slug: \"e2e\", entity_type: 0 }\n");

    // Create database.
    Command::cargo_bin("stacks")?
        .args([
            "client",
            "--url",
            "http://127.0.0.1:8000",
            "create_database",
            "e2e/test.sqlite",
            "sqlite",            
        ])
        .assert()
        .success()
        .stdout("Response is: InstantiatedDatabase { id: 1, entity_id: 1, slug: \"test.sqlite\", db_type: 0 }\n");

    // Populate and query database.
    client_query(
        "CREATE TABLE test_table(fname varchar, lname varchar);",
        "Response is: QueryResult { fields: [], rows: [] }",
    )?;
    client_query(
        "INSERT INTO test_table (fname, lname) VALUES (\"the first\", \"the last\");",
        "Response is: QueryResult { fields: [], rows: [] }",
    )?;
    client_query(
        "INSERT INTO test_table (fname, lname) VALUES (\"the first2\", \"the last2\");",
        "Response is: QueryResult { fields: [], rows: [] }",
    )?;
    client_query("SELECT * FROM test_table;",
                 "Response is: QueryResult { fields: [\"fname\", \"lname\"], rows: [[\"the first\", \"the last\"], [\"the first2\", \"the last2\"]] }")?;

    // TODO(marcua): Make this cleanup code run even on test failure.
    // See https://medium.com/@ericdreichert/test-setup-and-teardown-in-rust-without-a-framework-ba32d97aa5ab
    if let Err(err) = fs::remove_dir_all("/tmp/stacks/e2e") {
        assert_eq!(format!("{}", err), "No such file or directory (os error 2)")
    }
    server.kill()?;

    Ok(())
}
