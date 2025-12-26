use crate::utils::ayb::{list_tokens, query, revoke_token};
use std::collections::HashMap;

pub fn test_tokens(
    config_path: &str,
    api_keys: &HashMap<String, Vec<String>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let first_entity_api_keys = api_keys
        .get(super::FIRST_ENTITY_SLUG)
        .expect("first entity api key should exist");
    let first_entity_key = first_entity_api_keys
        .first()
        .expect("first entity should have at least one key");

    // Test listing tokens - should show the current token
    list_tokens(config_path, first_entity_key, "table", "ayb_")?;

    // Extract the short token from the full token for revoke test
    // Token format is: ayb_<short>_<secret>
    let short_token = first_entity_key
        .split('_')
        .take(2)
        .collect::<Vec<&str>>()
        .join("_");

    // If we have a second key, test revoking it
    if first_entity_api_keys.len() > 1 {
        let second_key = &first_entity_api_keys[1];
        let second_short_token = second_key
            .split('_')
            .take(2)
            .collect::<Vec<&str>>()
            .join("_");

        // Revoke the second token
        revoke_token(
            config_path,
            first_entity_key,
            &second_short_token,
            &format!("Successfully revoked token {second_short_token}"),
        )?;

        // Try to use the revoked token - should fail
        let result = query(
            config_path,
            second_key,
            "SELECT 1",
            super::FIRST_ENTITY_DB,
            "table",
            "Error: API token has been revoked",
        );
        assert!(result.is_ok(), "Query with revoked token should fail");
    }

    // List tokens again - should show current token only (minus any revoked ones)
    list_tokens(config_path, first_entity_key, "table", &short_token)?;

    println!("Token tests passed successfully");
    Ok(())
}
