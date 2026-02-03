use crate::utils::ayb::{list_tokens, query, revoke_token};
use std::collections::{HashMap, HashSet};

// Note: Scoped token permission capping tests are in oauth_tests.rs

/// Extract the short token from a full API key.
/// Token format is: ayb_<short_token>_<secret>
/// Returns just the <short_token> part (no prefix).
fn extract_short_token(full_token: &str) -> String {
    let parts: Vec<&str> = full_token.split('_').collect();
    // parts[0] = "ayb", parts[1] = short_token, parts[2..] = secret
    parts
        .get(1)
        .expect("token should have short token part")
        .to_string()
}

pub fn test_token_management(
    config_path: &str,
    api_keys: &HashMap<String, Vec<String>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let first_entity_api_keys = api_keys
        .get("first")
        .expect("first entity api key should exist");

    // We need at least 2 keys to test revocation
    assert!(
        first_entity_api_keys.len() >= 2,
        "first entity should have at least 2 API keys for revocation test"
    );

    let first_key = &first_entity_api_keys[0];
    let second_key = &first_entity_api_keys[1];

    // Extract short token for the second key (without ayb_ prefix)
    let second_short_token = extract_short_token(second_key);

    // Test 1: List tokens - verify we see exactly all expected tokens
    // The list should contain the short tokens (without ayb_ prefix)
    let token_list = list_tokens(config_path, first_key)?;
    let actual_tokens: HashSet<String> = token_list.into_iter().collect();
    let expected_tokens: HashSet<String> = first_entity_api_keys
        .iter()
        .map(|k| extract_short_token(k))
        .collect();
    assert_eq!(
        actual_tokens, expected_tokens,
        "Token list should contain exactly the expected tokens"
    );

    // Test 2: Verify second token works BEFORE revocation
    query(
        config_path,
        second_key,
        "SELECT 1",
        super::FIRST_ENTITY_DB,
        "table",
        "Rows: 1", // Should succeed and return one row
    )?;

    // Test 3: Revoke the second token
    revoke_token(
        config_path,
        first_key,
        &second_short_token,
        &format!("Successfully revoked token {second_short_token}"),
    )?;

    // Test 4: Verify revoked token no longer works
    query(
        config_path,
        second_key,
        "SELECT 1",
        super::FIRST_ENTITY_DB,
        "table",
        "Error: API token has been revoked",
    )?;

    // Test 5: List tokens again - second token should be gone
    let token_list_after = list_tokens(config_path, first_key)?;
    let actual_tokens_after: HashSet<String> = token_list_after.into_iter().collect();

    // Build expected set: all tokens except the revoked second token
    let expected_tokens_after: HashSet<String> = first_entity_api_keys
        .iter()
        .map(|k| extract_short_token(k))
        .filter(|t| t != &second_short_token)
        .collect();

    assert_eq!(
        actual_tokens_after, expected_tokens_after,
        "Token list after revocation should contain exactly the non-revoked tokens"
    );

    println!("Token management tests passed successfully");
    Ok(())
}
