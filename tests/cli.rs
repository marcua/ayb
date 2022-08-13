use assert_cmd::prelude::*;
use std::fs;
use std::process::Command;

#[test]
fn insert_and_read_rows() -> Result<(), Box<dyn std::error::Error>> {
    // Delete SQLite files from old runs (it's OK if they don't exist)
    match fs::remove_file("test.sqlite") {
        Ok(()) => {}
        Err(err) => {
            assert_eq!(format!("{}", err), "No such file or directory (os error 2)")
        }
    }

    Command::cargo_bin("stacks")?
        .args([
            "query",
            "--path",
            "test.sqlite",
            "--type",
            "sqlite",
            "--query",
            "CREATE TABLE test_table(fname varchar, lname varchar);",
        ])
        .assert()
        .success()
        .stdout("Result schema: []\nResults: []\n");

    Command::cargo_bin("stacks")?
        .args([
            "query",
            "--path",
            "test.sqlite",
            "--type",
            "sqlite",
            "--query",
            "INSERT INTO test_table (fname, lname) VALUES (\"the first\", \"the last\");",
        ])
        .assert()
        .success()
        .stdout("Result schema: []\nResults: []\n");

    Command::cargo_bin("stacks")?
        .args([
            "query",
            "--path",
            "test.sqlite",
            "--type",
            "sqlite",
            "--query",
            "INSERT INTO test_table (fname, lname) VALUES (\"the first2\", \"the last2\");",
        ])
        .assert()
        .success()
        .stdout("Result schema: []\nResults: []\n");

    let expected = r#"Result schema: [
    "fname",
    "lname",
]
Results!!!: [
    [
        "the first",
        "the last",
    ],
    [
        "the first2",
        "the last2",
    ],
]
"#;

    Command::cargo_bin("stacks")?
        .args([
            "query",
            "--path",
            "test.sqlite",
            "--type",
            "sqlite",
            "--query",
            "SELECT * FROM test_table;",
        ])
        .assert()
        .success()
        .stdout(expected);

    fs::remove_file("test.sqlite").expect("Unable to clean up test.sqlite");

    Ok(())
}
