use crate::e2e_tests::{
    FIRST_ENTITY_DB, FIRST_ENTITY_DB2, FIRST_ENTITY_DB_CASED, FIRST_ENTITY_DUCKDB,
};
use crate::utils::ayb::{
    create_database, create_database_with_type, query, query_no_api_token, set_default_url,
};
use ayb::client::config::ClientConfig;
use std::collections::HashMap;
use std::fs;

pub fn test_create_and_query_db(
    config_path: &str,
    api_keys: &HashMap<String, Vec<String>>,
    server_url: &str,
    expected_config: &mut ClientConfig,
) -> Result<(), Box<dyn std::error::Error>> {
    // Can't create database on e2e-first with e2e-second's token.
    create_database(
        config_path,
        &api_keys.get("second").unwrap()[0],
        FIRST_ENTITY_DB,
        "Error: Authenticated entity e2e-second can't create a database for entity e2e-first",
    )?;

    // Can't create database on e2e-first with invalid token.
    create_database(
        config_path,
        &format!("{}bad", api_keys.get("first").unwrap()[0]),
        FIRST_ENTITY_DB,
        "Error: Invalid API token",
    )?;

    // Create a database with the appropriate user/key pair.
    create_database(
        config_path,
        &api_keys.get("first").unwrap()[0],
        FIRST_ENTITY_DB,
        "Successfully created e2e-first/test.sqlite",
    )?;

    // Can't create a database twice.
    create_database(
        config_path,
        &api_keys.get("first").unwrap()[0],
        FIRST_ENTITY_DB,
        "Error: Database already exists",
    )?;

    // Can create another database with the appropriate user/key pair.
    create_database(
        config_path,
        &api_keys.get("first").unwrap()[0],
        FIRST_ENTITY_DB2,
        "Successfully created e2e-first/another.sqlite",
    )?;

    // Can't query database with second account's API key
    query(
        config_path,
        &api_keys.get("second").unwrap()[0],
        "CREATE TABLE test_table(fname varchar, lname varchar);",
        FIRST_ENTITY_DB,
        "table",
        "Error: Authenticated entity e2e-second can't query database e2e-first/test.sqlite",
    )?;

    // Can't query database with bad API key.
    query(
        config_path,
        &format!("{}bad", api_keys.get("first").unwrap()[0]),
        "CREATE TABLE test_table(fname varchar, lname varchar);",
        FIRST_ENTITY_DB,
        "table",
        "Error: Invalid API token",
    )?;

    // Populate and query database. Alternate through the three API
    // keys for the first account to ensure they all work.
    query(
        config_path,
        &api_keys.get("first").unwrap()[0],
        "CREATE TABLE test_table(fname varchar, lname varchar);",
        FIRST_ENTITY_DB,
        "table",
        "\nRows: 0",
    )?;
    query(
        config_path,
        &api_keys.get("first").unwrap()[1],
        "INSERT INTO test_table (fname, lname) VALUES (\"the first\", \"the last\");",
        FIRST_ENTITY_DB,
        "table",
        "\nRows: 0",
    )?;
    query(
        config_path,
        &api_keys.get("first").unwrap()[2],
        "INSERT INTO test_table (fname, lname) VALUES (\"the first2\", \"the last2\");",
        FIRST_ENTITY_DB,
        "table",
        "\nRows: 0",
    )?;
    query(
        config_path,
        &api_keys.get("first").unwrap()[0],
        "SELECT * FROM test_table;",
        FIRST_ENTITY_DB,
        "table",                 
        " fname      | lname \n------------+-----------\n the first  | the last \n the first2 | the last2 \n\nRows: 2")?;
    query(
        config_path,
        &api_keys.get("first").unwrap()[0],
        "SELECT * FROM test_table;",
        FIRST_ENTITY_DB,
        "csv",
        "fname,lname\nthe first,the last\nthe first2,the last2\n\nRows: 2",
    )?;

    // Querying with no API token also works, because the first
    // account token is saved in the configuration file.
    query_no_api_token(
        config_path,
        "SELECT * FROM test_table;",
        FIRST_ENTITY_DB,
        "csv",
        "fname,lname\nthe first,the last\nthe first2,the last2\n\nRows: 2",
    )?;

    // We now test setting the default server URL: we set it to
    // something nonsensical and it breaks connections, and when we
    // reset it, the connections work again.
    set_default_url(
        config_path,
        &format!("{server_url}badport"),
        &format!("Saved {server_url}badport as new default_url"),
    )?;
    expected_config.default_url = Some(format!("{server_url}badport"));
    assert_eq!(
        fs::read_to_string(config_path).unwrap(),
        serde_json::to_string(&expected_config)?
    );
    query(
        config_path,
        &api_keys.get("first").unwrap()[0],
        "SELECT * FROM test_table;",
        FIRST_ENTITY_DB,
        "csv",
        "Error: reqwest::Error { kind: Builder, source: InvalidPort }",
    )?;
    set_default_url(
        config_path,
        server_url,
        &format!("Saved {server_url} as new default_url"),
    )?;
    expected_config.default_url = Some(server_url.to_string());
    assert_eq!(
        fs::read_to_string(config_path).unwrap(),
        serde_json::to_string(&expected_config)?
    );
    query(
        config_path,
        &api_keys.get("first").unwrap()[0],
        "SELECT * FROM test_table;",
        FIRST_ENTITY_DB_CASED, // Entity slugs should be case-insensitive
        "csv",
        "fname,lname\nthe first,the last\nthe first2,the last2\n\nRows: 2",
    )?;

    Ok(())
}

pub fn test_create_and_query_duckdb(
    config_path: &str,
    api_keys: &HashMap<String, Vec<String>>,
) -> Result<(), Box<dyn std::error::Error>> {
    create_database_with_type(
        config_path,
        &api_keys.get("first").unwrap()[0],
        FIRST_ENTITY_DUCKDB,
        "duckdb",
        "Successfully created e2e-first/test.duckdb",
    )?;

    query(
        config_path,
        &api_keys.get("first").unwrap()[0],
        "CREATE TABLE duck_test(id INTEGER, name VARCHAR);",
        FIRST_ENTITY_DUCKDB,
        "table",
        "\nRows: 0",
    )?;
    query(
        config_path,
        &api_keys.get("first").unwrap()[0],
        "INSERT INTO duck_test VALUES (1, 'hello'), (2, 'world');",
        FIRST_ENTITY_DUCKDB,
        "table",
        "\nRows: 0",
    )?;
    query(
        config_path,
        &api_keys.get("first").unwrap()[0],
        "SELECT * FROM duck_test ORDER BY id;",
        FIRST_ENTITY_DUCKDB,
        "csv",
        "id,name\n1,hello\n2,world\n\nRows: 2",
    )?;

    Ok(())
}
