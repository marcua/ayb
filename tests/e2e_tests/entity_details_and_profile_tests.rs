use crate::e2e_tests::{FIRST_ENTITY_SLUG, FIRST_ENTITY_SLUG_CASED};
use crate::utils::ayb::{list_databases, profile, update_profile};
use std::collections::HashMap;

pub fn test_entity_details_and_profile(
    config_path: &str,
    api_keys: &HashMap<String, Vec<String>>,
) -> Result<(), Box<dyn std::error::Error>> {
    // List databases from first account using its API key
    list_databases(
        config_path,
        &api_keys.get("first").unwrap()[0],
        FIRST_ENTITY_SLUG_CASED, // Entity slugs should be case-insensitive
        "csv",
        "Database slug,Type\nanother.sqlite,sqlite\ntest.sqlite,sqlite",
    )?;

    // List databases from first account using the API key of the second account
    list_databases(
        config_path,
        &api_keys.get("second").unwrap()[0],
        FIRST_ENTITY_SLUG,
        "csv",
        &format!("No queryable databases owned by {FIRST_ENTITY_SLUG}"),
    )?;

    // Make some partial profile updates and verify profile details upon retrieval
    update_profile(
        config_path,
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
        config_path,
        &api_keys.get("first").unwrap()[0],
        FIRST_ENTITY_SLUG_CASED, // Entity slugs should be case-insensitive
        "csv",
        "Display name,Description,Organization,Location,Links\nEntity 0,null,null,null,",
    )?;

    update_profile(
        config_path,
        &api_keys.get("first").unwrap()[0],
        FIRST_ENTITY_SLUG_CASED, // Entity slugs should be case-insensitive
        Some("Entity 0"),
        Some("Entity 0 description"),
        None,
        None,
        None,
        "Successfully updated profile",
    )?;

    profile(
        config_path,
        &api_keys.get("first").unwrap()[0],
        FIRST_ENTITY_SLUG,
        "csv",
        "Display name,Description,Organization,Location,Links\nEntity 0,Entity 0 description,null,null,"
    )?;

    profile(
        config_path,
        &api_keys.get("second").unwrap()[0],
        FIRST_ENTITY_SLUG,
        "csv",
        "Display name,Description,Organization,Location,Links\nEntity 0,Entity 0 description,null,null,"
    )?;

    update_profile(
        config_path,
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
        config_path,
        &api_keys.get("first").unwrap()[0],
        FIRST_ENTITY_SLUG,
        "csv",
        "Display name,Description,Organization,Location,Links\nEntity 0,Entity 0 NEW description,Entity 0 organization,null,\"http://ayb.host/,http://ayb2.host\""
    )?;

    profile(
        config_path,
        &api_keys.get("second").unwrap()[0],
        FIRST_ENTITY_SLUG,
        "csv",
        "Display name,Description,Organization,Location,Links\nEntity 0,Entity 0 NEW description,Entity 0 organization,null,\"http://ayb.host/,http://ayb2.host\""
    )?;

    // Bidi and invisible formatting characters are stripped when the
    // profile is written, so a display name can't reorder the text
    // rendered around it. Each field below cleans up to the value it
    // already holds, which leaves the profile unchanged.
    update_profile(
        config_path,
        &api_keys.get("first").unwrap()[0],
        FIRST_ENTITY_SLUG,
        Some("Entity\u{202E} 0"),
        Some("Entity 0\u{200B} NEW description"),
        Some("Entity 0 organi\u{00AD}zation"),
        None,
        Some(vec!["http://ayb.host/\u{2066}", "http://ayb2.host"]),
        "Successfully updated profile",
    )?;

    profile(
        config_path,
        &api_keys.get("first").unwrap()[0],
        FIRST_ENTITY_SLUG,
        "csv",
        "Display name,Description,Organization,Location,Links\nEntity 0,Entity 0 NEW description,Entity 0 organization,null,\"http://ayb.host/,http://ayb2.host\""
    )?;

    // Names in other scripts, which is what these fields are for, pass
    // through untouched.
    update_profile(
        config_path,
        &api_keys.get("first").unwrap()[0],
        FIRST_ENTITY_SLUG,
        Some("أحمد 北京 José"),
        None,
        None,
        None,
        None,
        "Successfully updated profile",
    )?;

    profile(
        config_path,
        &api_keys.get("first").unwrap()[0],
        FIRST_ENTITY_SLUG,
        "csv",
        "Display name,Description,Organization,Location,Links\nأحمد 北京 José,Entity 0 NEW description,Entity 0 organization,null,\"http://ayb.host/,http://ayb2.host\""
    )?;

    // Restore the display name for any test that runs after this one.
    update_profile(
        config_path,
        &api_keys.get("first").unwrap()[0],
        FIRST_ENTITY_SLUG,
        Some("Entity 0"),
        None,
        None,
        None,
        None,
        "Successfully updated profile",
    )?;

    // Test that update_profile with no arguments returns an error
    update_profile(
        config_path,
        &api_keys.get("first").unwrap()[0],
        FIRST_ENTITY_SLUG,
        None,
        None,
        None,
        None,
        None,
        "Error: No fields provided to update. Please specify at least one field to update.",
    )?;

    Ok(())
}
