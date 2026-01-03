use crate::utils::ayb::{list_tokens, list_tokens_json, query, revoke_token};
use std::collections::HashMap;

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
    let third_key = first_entity_api_keys.get(2);

    // Extract short tokens (without ayb_ prefix)
    let first_short_token = extract_short_token(first_key);
    let second_short_token = extract_short_token(second_key);

    // Test 1: List tokens - verify we see all expected tokens
    // The list should contain the short tokens (without ayb_ prefix)
    let token_list = list_tokens_json(config_path, first_key)?;
    assert!(
        token_list.contains(&first_short_token),
        "Token list should contain first token: {}",
        first_short_token
    );
    assert!(
        token_list.contains(&second_short_token),
        "Token list should contain second token: {}",
        second_short_token
    );
    if let Some(third) = third_key {
        let third_short_token = extract_short_token(third);
        assert!(
            token_list.contains(&third_short_token),
            "Token list should contain third token: {}",
            third_short_token
        );
    }

    // Verify row count matches expected tokens
    let expected_count = first_entity_api_keys.len();
    let actual_count = token_list.len();
    assert_eq!(
        actual_count, expected_count,
        "Token list should have {} tokens, found {}",
        expected_count, actual_count
    );

    // Test 2: Verify second token works BEFORE revocation
    query(
        config_path,
        second_key,
        "SELECT 1",
        super::FIRST_ENTITY_DB,
        "table",
        "1", // Should succeed and return result
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
    let token_list_after = list_tokens_json(config_path, first_key)?;

    // First token should still be present
    assert!(
        token_list_after.contains(&first_short_token),
        "Token list should still contain first token after revocation"
    );

    // Second token should be gone
    assert!(
        !token_list_after.contains(&second_short_token),
        "Token list should NOT contain revoked second token"
    );

    // Verify correct count after revocation
    let expected_after = expected_count - 1;
    assert_eq!(
        token_list_after.len(),
        expected_after,
        "Token list should have {} tokens after revocation, found {}",
        expected_after,
        token_list_after.len()
    );

    // Also verify with table format for visual confirmation
    list_tokens(config_path, first_key, "table", &first_short_token)?;

    println!("Token management tests passed successfully");
    Ok(())
}
