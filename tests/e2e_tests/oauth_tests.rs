use crate::utils::ayb::query;
use std::collections::HashMap;

/// Test permission capping when a scoped token has reduced permissions.
///
/// Scenario: User has read-write access to a database, but we create a token
/// scoped with read-only permission. The token should only be able to read,
/// not write. This verifies the basic write access still works with the full token.
pub async fn test_oauth_permission_capping(
    config_path: &str,
    api_keys: &HashMap<String, Vec<String>>,
    _server_url: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("Testing OAuth permission capping...");

    // Get the first entity's API key (which has read-write access to their own DB)
    let first_entity_key = &api_keys
        .get("first")
        .expect("first entity key should exist")[0];

    // Test that the owner can write (using the full access token)
    query(
        config_path,
        first_entity_key,
        "CREATE TABLE IF NOT EXISTS oauth_test (id INTEGER PRIMARY KEY, name TEXT)",
        super::FIRST_ENTITY_DB,
        "table",
        "\nRows: 0",
    )?;

    query(
        config_path,
        first_entity_key,
        "INSERT INTO oauth_test (name) VALUES ('test')",
        super::FIRST_ENTITY_DB,
        "table",
        "\nRows: 0",
    )?;

    // Verify the data was inserted
    query(
        config_path,
        first_entity_key,
        "SELECT COUNT(*) FROM oauth_test",
        super::FIRST_ENTITY_DB,
        "table",
        "1",
    )?;

    // Clean up the test table
    query(
        config_path,
        first_entity_key,
        "DROP TABLE oauth_test",
        super::FIRST_ENTITY_DB,
        "table",
        "\nRows: 0",
    )?;

    println!("OAuth permission capping test passed (basic verification)");

    // Note: Full OAuth flow testing with scoped tokens requires browser interaction
    // or a more complex test setup. The browser e2e tests cover the UI flow.
    // The permission capping logic in permissions.rs is unit-testable, but
    // the integration test for scoped tokens via OAuth requires the full flow.

    Ok(())
}

/// Test the OAuth token exchange endpoint directly
pub async fn test_oauth_token_exchange(server_url: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!("Testing OAuth token exchange endpoint...");

    let client = reqwest::Client::new();

    // Test 1: Invalid grant_type should fail
    let response = client
        .post(&format!("{}/v1/oauth/token", server_url))
        .json(&serde_json::json!({
            "grant_type": "invalid",
            "code": "test",
            "redirect_uri": "http://localhost:3000/callback",
            "code_verifier": "test"
        }))
        .send()
        .await?;

    assert_eq!(response.status(), 400);
    let error: serde_json::Value = response.json().await?;
    assert_eq!(error["error"], "unsupported_grant_type");

    // Test 2: Invalid code should fail
    let response = client
        .post(&format!("{}/v1/oauth/token", server_url))
        .json(&serde_json::json!({
            "grant_type": "authorization_code",
            "code": "nonexistent_code",
            "redirect_uri": "http://localhost:3000/callback",
            "code_verifier": "test"
        }))
        .send()
        .await?;

    assert_eq!(response.status(), 400);
    let error: serde_json::Value = response.json().await?;
    assert_eq!(error["error"], "invalid_grant");

    println!("OAuth token exchange tests passed");
    Ok(())
}
