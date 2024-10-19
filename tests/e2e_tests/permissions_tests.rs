use crate::e2e_tests::FIRST_ENTITY_DB;
use crate::utils::ayb::{query, update_database};
use std::collections::HashMap;

pub async fn test_permissions(
    config_path: &str,
    api_keys: &HashMap<String, Vec<String>>,
) -> Result<(), Box<dyn std::error::Error>> {
    // While first entity has access to database (it's the owner), the
    // second one can't query or find it in list.
    query(
        &config_path,
        &api_keys.get("first").unwrap()[1],
        "SELECT COUNT(*) AS the_count FROM test_table;",
        FIRST_ENTITY_DB,
        "table",
        " the_count \n-----------\n 3 \n\nRows: 1",
    )?;
    query(
        &config_path,
        &api_keys.get("second").unwrap()[0],
        "SELECT COUNT(*) AS the_count FROM test_table;",
        FIRST_ENTITY_DB,
        "table",
        "Error: Authenticated entity e2e-second can't query database e2e-first/test.sqlite",
    )?;

    //TODO(marcua): Implement "can't find database in list" part.

    // Add metadata-only access, ensure you can list but not query
    // Second entity can't update database, but first can.
    update_database(
        &config_path,
        &api_keys.get("second").unwrap()[0],
        FIRST_ENTITY_DB,
        "metadata",
        "Error: Authenticated entity e2e-second can't update database e2e-first/test.sqlite",
    )?;

    update_database(
        &config_path,
        &api_keys.get("first").unwrap()[0],
        FIRST_ENTITY_DB,
        "metadata",
        "Database e2e-first/test.sqlite updated successfully",
    )?;

    // TODO(marcua): Implement list, no query checks

    // TODO(marcua): When we support forking, add forking tests.

    // Add public read-only permissions, ensure entity can select but not insert
    update_database(
        &config_path,
        &api_keys.get("first").unwrap()[0],
        FIRST_ENTITY_DB,
        "read-only",
        "Database e2e-first/test.sqlite updated successfully",
    )?;

    // TODO(marcua): Implement list and query checks

    // Remove permissions, ensure entity can't query or find database in list
    update_database(
        &config_path,
        &api_keys.get("first").unwrap()[0],
        FIRST_ENTITY_DB,
        "no-access",
        "Database e2e-first/test.sqlite updated successfully",
    )?;

    // TODO(marcua): Implement no list and no query checks

    // TODO(marcua): When ready, test entity-database permissions.
    Ok(())
}
