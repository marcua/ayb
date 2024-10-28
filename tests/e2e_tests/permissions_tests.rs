use crate::e2e_tests::{FIRST_ENTITY_DB, FIRST_ENTITY_SLUG};
use crate::utils::ayb::{list_databases, query, update_database};
use std::collections::HashMap;

pub async fn test_permissions(
    config_path: &str,
    api_keys: &HashMap<String, Vec<String>>,
) -> Result<(), Box<dyn std::error::Error>> {
    // While first entity has query access to database and can find it
    // in a list (it's the owner), the second one can't do either.
    query(
        &config_path,
        &api_keys.get("first").unwrap()[1],
        "INSERT INTO test_table (fname, lname) VALUES (\"first permissions1\", \"last permissions1\");",
        FIRST_ENTITY_DB,
        "table",
        "\nRows: 0",
    )?;
    list_databases(
        &config_path,
        &api_keys.get("first").unwrap()[0],
        FIRST_ENTITY_SLUG,
        "csv",
        "Database slug,Type\ntest.sqlite,sqlite",
    )?;
    query(
        &config_path,
        &api_keys.get("second").unwrap()[0],
        "SELECT COUNT(*) AS the_count FROM test_table;",
        FIRST_ENTITY_DB,
        "table",
        "Error: Authenticated entity e2e-second can't query database e2e-first/test.sqlite",
    )?;
    list_databases(
        &config_path,
        &api_keys.get("second").unwrap()[0],
        FIRST_ENTITY_SLUG,
        "csv",
        &format!("No queryable databases owned by {}", FIRST_ENTITY_SLUG),
    )?;

    // Second entity can't update database, but first can.
    update_database(
        &config_path,
        &api_keys.get("second").unwrap()[0],
        FIRST_ENTITY_DB,
        "fork",
        "Error: Authenticated entity e2e-second can't update database e2e-first/test.sqlite",
    )?;
    update_database(
        &config_path,
        &api_keys.get("first").unwrap()[0],
        FIRST_ENTITY_DB,
        "fork",
        "Database e2e-first/test.sqlite updated successfully",
    )?;

    // With fork-level access, the second entity can't query the database, but can discover it.
    query(
        &config_path,
        &api_keys.get("second").unwrap()[0],
        "SELECT COUNT(*) AS the_count FROM test_table;",
        FIRST_ENTITY_DB,
        "table",
        "Error: Authenticated entity e2e-second can't query database e2e-first/test.sqlite",
    )?;
    list_databases(
        &config_path,
        &api_keys.get("second").unwrap()[0],
        FIRST_ENTITY_SLUG,
        "csv",
        "Database slug,Type\ntest.sqlite,sqlite",
    )?;
    // TODO(marcua): When we implement forking, test that second
    // entity can fork now, but not before the permission was granted.

    // With public read-only permissions, the second entity can issue
    // read-only (SELECT) queries, but not modify the database (e.g.,
    // INSERT). It should also still be able to discover the datbaase.
    update_database(
        &config_path,
        &api_keys.get("first").unwrap()[0],
        FIRST_ENTITY_DB,
        "read-only",
        "Database e2e-first/test.sqlite updated successfully",
    )?;
    query(
        &config_path,
        &api_keys.get("second").unwrap()[0],
        "SELECT COUNT(*) AS the_count FROM test_table;",
        FIRST_ENTITY_DB,
        "table",
        " the_count \n-----------\n 4 \n\nRows: 1",
    )?;
    query(
        &config_path,
        &api_keys.get("second").unwrap()[0],
        "INSERT INTO test_table (fname, lname) VALUES (\"first permissions2\", \"last permissions2\");",        
        FIRST_ENTITY_DB,
        "table",
        "Error: Attempted to write to a read-only database",
    )?;
    list_databases(
        &config_path,
        &api_keys.get("second").unwrap()[0],
        FIRST_ENTITY_SLUG,
        "csv",
        "Database slug,Type\ntest.sqlite,sqlite",
    )?;

    // With no public permissions, the second entity can't query or discover the database.
    update_database(
        &config_path,
        &api_keys.get("first").unwrap()[0],
        FIRST_ENTITY_DB,
        "no-access",
        "Database e2e-first/test.sqlite updated successfully",
    )?;
    query(
        &config_path,
        &api_keys.get("second").unwrap()[0],
        "SELECT COUNT(*) AS the_count FROM test_table;",
        FIRST_ENTITY_DB,
        "table",
        "Error: Authenticated entity e2e-second can't query database e2e-first/test.sqlite",
    )?;
    list_databases(
        &config_path,
        &api_keys.get("second").unwrap()[0],
        FIRST_ENTITY_SLUG,
        "csv",
        &format!("No queryable databases owned by {}", FIRST_ENTITY_SLUG),
    )?;

    // TODO(marcua): When ready, test entity-database permissions.
    Ok(())
}
