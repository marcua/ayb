use crate::e2e_tests::{FIRST_ENTITY_DB, FIRST_ENTITY_SLUG};
use crate::utils::testing::get_pgwire_port;
use std::collections::HashMap;
use tokio_postgres::{Error as PgError, NoTls};

/// Test pgwire queries using authenticated and unauthenticated connections.
/// This runs after the database has been created and populated by create_and_query_db_tests.
pub async fn test_pgwire_queries(
    test_type: &str,
    api_keys: &HashMap<String, Vec<String>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let pgwire_port = get_pgwire_port(test_type)?;
    let api_token = &api_keys.get("first").unwrap()[0];

    // Test 1: Authenticated query should succeed
    test_authenticated_query(pgwire_port, FIRST_ENTITY_SLUG, FIRST_ENTITY_DB, api_token).await?;

    // Test 2: Unauthenticated query (wrong password) should fail
    test_unauthenticated_query_wrong_password(pgwire_port, FIRST_ENTITY_SLUG, FIRST_ENTITY_DB)
        .await?;

    // Test 3: Wrong user for token should fail
    test_wrong_user_for_token(pgwire_port, "wrong-user", FIRST_ENTITY_DB, api_token).await?;

    Ok(())
}

/// Test that an authenticated user can query the database via pgwire
async fn test_authenticated_query(
    port: u16,
    username: &str,
    database: &str,
    api_token: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let conn_string = format!(
        "host=127.0.0.1 port={} user={} password={} dbname={}",
        port, username, api_token, database
    );

    let (client, connection) = tokio_postgres::connect(&conn_string, NoTls).await?;

    // Spawn the connection to run in the background
    tokio::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!("pgwire connection error: {}", e);
        }
    });

    // Query the test_table that was created by create_and_query_db_tests
    let rows = client.query("SELECT * FROM test_table", &[]).await?;

    // Verify we got the expected 2 rows
    assert_eq!(
        rows.len(),
        2,
        "Expected 2 rows from test_table via pgwire, got {}",
        rows.len()
    );

    // Verify the data content
    let first_row_fname: &str = rows[0].get(0);
    let first_row_lname: &str = rows[0].get(1);
    assert_eq!(first_row_fname, "the first");
    assert_eq!(first_row_lname, "the last");

    let second_row_fname: &str = rows[1].get(0);
    let second_row_lname: &str = rows[1].get(1);
    assert_eq!(second_row_fname, "the first2");
    assert_eq!(second_row_lname, "the last2");

    println!(
        "pgwire authenticated query test passed: got {} rows",
        rows.len()
    );
    Ok(())
}

/// Test that a query with wrong password fails
async fn test_unauthenticated_query_wrong_password(
    port: u16,
    username: &str,
    database: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let conn_string = format!(
        "host=127.0.0.1 port={} user={} password={} dbname={}",
        port, username, "wrong_password", database
    );

    let result = tokio_postgres::connect(&conn_string, NoTls).await;

    match result {
        Ok(_) => {
            return Err("Expected connection to fail with wrong password, but it succeeded".into());
        }
        Err(e) => {
            // The error should indicate authentication failure
            let error_string = e.to_string();
            assert!(
                error_string.contains("Invalid API token")
                    || error_string.contains("authentication")
                    || is_connection_refused(&e),
                "Expected authentication error, got: {}",
                error_string
            );
            println!(
                "pgwire unauthenticated query test passed: connection correctly rejected with: {}",
                error_string
            );
        }
    }

    Ok(())
}

/// Test that using a token belonging to a different user fails
async fn test_wrong_user_for_token(
    port: u16,
    username: &str,
    database: &str,
    api_token: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let conn_string = format!(
        "host=127.0.0.1 port={} user={} password={} dbname={}",
        port, username, api_token, database
    );

    let result = tokio_postgres::connect(&conn_string, NoTls).await;

    match result {
        Ok(_) => {
            return Err(
                "Expected connection to fail with wrong user for token, but it succeeded".into(),
            );
        }
        Err(e) => {
            let error_string = e.to_string();
            assert!(
                error_string.contains("Token belongs to entity")
                    || error_string.contains("authentication")
                    || is_connection_refused(&e),
                "Expected token/user mismatch error, got: {}",
                error_string
            );
            println!(
                "pgwire wrong user test passed: connection correctly rejected with: {}",
                error_string
            );
        }
    }

    Ok(())
}

/// Check if the error is a connection refused error (server might have closed connection)
fn is_connection_refused(e: &PgError) -> bool {
    if let Some(io_err) = e.as_db_error() {
        return io_err.message().contains("refused") || io_err.message().contains("closed");
    }
    e.to_string().contains("refused") || e.to_string().contains("closed")
}
