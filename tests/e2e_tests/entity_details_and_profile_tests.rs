use crate::e2e_tests::FIRST_ENTITY_SLUG;
use crate::utils::ayb::{list_databases, profile, update_profile};
use std::collections::HashMap;

pub fn test_entity_details_and_profile(
    config_path: &str,
    api_keys: &HashMap<String, Vec<String>>,
) -> Result<(), Box<dyn std::error::Error>> {
    // List databases from first account using its API key
    list_databases(
        &config_path,
        &api_keys.get("first").unwrap()[0],
        "E2E-FiRsT", // Entity slugs should be case-insensitive
        "csv",
        "Database slug,Type\ntest.sqlite,sqlite",
    )?;

    // List databases from first account using the API key of the second account
    list_databases(
        &config_path,
        &api_keys.get("second").unwrap()[0],
        FIRST_ENTITY_SLUG,
        "csv",
        &format!("No queryable databases owned by {}", FIRST_ENTITY_SLUG),
    )?;

    // Make some partial profile updates and verify profile details upon retrieval
    update_profile(
        &config_path,
        &api_keys.get("first").unwrap()[0],
        FIRST_ENTITY_SLUG,
        Some("Entity 0"),
        None,
        None,
        None,
        None,
        "Successfully updated profile",
    )?;

    profile(
        &config_path,
        &api_keys.get("first").unwrap()[0],
        "E2E-FiRsT", // Entity slugs should be case-insensitive
        "csv",
        "Display name,Description,Organization,Location,Links\nEntity 0,null,null,null,",
    )?;

    update_profile(
        &config_path,
        &api_keys.get("first").unwrap()[0],
        "E2E-FiRST", // Entity slugs should be case-insensitive
        Some("Entity 0"),
        Some("Entity 0 description"),
        None,
        None,
        None,
        "Successfully updated profile",
    )?;

    profile(
        &config_path,
        &api_keys.get("first").unwrap()[0],
        FIRST_ENTITY_SLUG,
        "csv",
        "Display name,Description,Organization,Location,Links\nEntity 0,Entity 0 description,null,null,"
    )?;

    profile(
        &config_path,
        &api_keys.get("second").unwrap()[0],
        FIRST_ENTITY_SLUG,
        "csv",
        "Display name,Description,Organization,Location,Links\nEntity 0,Entity 0 description,null,null,"
    )?;

    update_profile(
        &config_path,
        &api_keys.get("first").unwrap()[0],
        FIRST_ENTITY_SLUG,
        None,
        Some("Entity 0 NEW description"),
        Some("Entity 0 organization"),
        None,
        Some(vec!["http://ayb.host/", "http://ayb2.host"]),
        "Successfully updated profile",
    )?;

    profile(
        &config_path,
        &api_keys.get("first").unwrap()[0],
        FIRST_ENTITY_SLUG,
        "csv",
        "Display name,Description,Organization,Location,Links\nEntity 0,Entity 0 NEW description,Entity 0 organization,null,\"http://ayb.host/,http://ayb2.host\""
    )?;

    profile(
        &config_path,
        &api_keys.get("second").unwrap()[0],
        FIRST_ENTITY_SLUG,
        "csv",
        "Display name,Description,Organization,Location,Links\nEntity 0,Entity 0 NEW description,Entity 0 organization,null,\"http://ayb.host/,http://ayb2.host\""
    )?;

    Ok(())
}
