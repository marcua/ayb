use crate::e2e_tests::{
    FIRST_ENTITY_DB, FIRST_ENTITY_DB2, FIRST_ENTITY_SLUG, SECOND_ENTITY_SLUG, THIRD_ENTITY_SLUG,
};
use crate::utils::ayb::{
    database_details, list_databases, list_database_permissions, list_snapshots, list_snapshots_match_output,
    query, share, update_database,
};
use std::collections::HashMap;

pub async fn test_permissions(
    config_path: &str,
    api_keys: &HashMap<String, Vec<String>>,
) -> Result<(), Box<dyn std::error::Error>> {
    // The first set of tests cover the various permissions afforded
    // by the public sharing level (global no-access/fork/read-only
    // permissions).

    // While first entity has query access to database and can find it
    // in a list (it's the owner), the second one can't do either. The
    // first database has full database details access, whereas the
    // second can't access the details.
    query(
        config_path,
        &api_keys.get("first").unwrap()[1],
        "INSERT INTO test_table (fname, lname) VALUES (\"first permissions1\", \"last permissions1\");",
        FIRST_ENTITY_DB,
        "table",
        "\nRows: 0",
    )?;
    list_databases(
        config_path,
        &api_keys.get("first").unwrap()[0],
        FIRST_ENTITY_SLUG,
        "csv",
        // Note that while the first entity can see another.sqlite,
        // the second/third entities, when granted access to
        // test.sqlite, will not be able to see another.sqlite.
        "Database slug,Type\nanother.sqlite,sqlite\ntest.sqlite,sqlite",
    )?;
    database_details(
        config_path,
        &api_keys.get("first").unwrap()[0],
        FIRST_ENTITY_DB,
        "Database: e2e-first/test.sqlite\nType: sqlite\nAccess level: ReadWrite\nYou have management permissions for this database",
    )?;
    query(
        config_path,
        &api_keys.get("second").unwrap()[0],
        "SELECT COUNT(*) AS the_count FROM test_table;",
        FIRST_ENTITY_DB,
        "table",
        "Error: Authenticated entity e2e-second can't query database e2e-first/test.sqlite",
    )?;
    list_databases(
        config_path,
        &api_keys.get("second").unwrap()[0],
        FIRST_ENTITY_SLUG,
        "csv",
        &format!("No queryable databases owned by {}", FIRST_ENTITY_SLUG),
    )?;
    database_details(
        config_path,
        &api_keys.get("second").unwrap()[0],
        FIRST_ENTITY_DB,
        "Error: Authenticated entity e2e-second can't access database e2e-first/test.sqlite",
    )?;

    // Second entity can't update database, but first can.
    update_database(
        config_path,
        &api_keys.get("second").unwrap()[0],
        FIRST_ENTITY_DB,
        "fork",
        "Error: Authenticated entity e2e-second can't update database e2e-first/test.sqlite",
    )?;
    update_database(
        config_path,
        &api_keys.get("first").unwrap()[0],
        FIRST_ENTITY_DB,
        "fork",
        "Database e2e-first/test.sqlite updated successfully",
    )?;

    // With fork-level access, the second entity can't query the database, but can discover it.
    query(
        config_path,
        &api_keys.get("second").unwrap()[0],
        "SELECT COUNT(*) AS the_count FROM test_table;",
        FIRST_ENTITY_DB,
        "table",
        "Error: Authenticated entity e2e-second can't query database e2e-first/test.sqlite",
    )?;
    list_databases(
        config_path,
        &api_keys.get("second").unwrap()[0],
        FIRST_ENTITY_SLUG,
        "csv",
        "Database slug,Type\ntest.sqlite,sqlite",
    )?;
    database_details(
        config_path,
        &api_keys.get("second").unwrap()[0],
        FIRST_ENTITY_DB,
        "Database: e2e-first/test.sqlite\nType: sqlite\nAccess level: No query access",
    )?;

    // TODO(marcua): When we implement forking, test that second
    // entity can fork now, but not before the permission was granted.

    // With public read-only permissions, the second entity can issue
    // read-only (SELECT) queries, but not modify the database (e.g.,
    // INSERT). It should also still be able to discover the database.
    update_database(
        config_path,
        &api_keys.get("first").unwrap()[0],
        FIRST_ENTITY_DB,
        "read-only",
        "Database e2e-first/test.sqlite updated successfully",
    )?;
    query(
        config_path,
        &api_keys.get("second").unwrap()[0],
        "SELECT COUNT(*) AS the_count FROM test_table;",
        FIRST_ENTITY_DB,
        "table",
        " the_count \n-----------\n 4 \n\nRows: 1",
    )?;
    query(
        config_path,
        &api_keys.get("second").unwrap()[0],
        "INSERT INTO test_table (fname, lname) VALUES (\"first permissions2\", \"last permissions2\");",
        FIRST_ENTITY_DB,
        "table",
        "Error: Attempted to write to database while in read-only mode",
    )?;
    list_databases(
        config_path,
        &api_keys.get("second").unwrap()[0],
        FIRST_ENTITY_SLUG,
        "csv",
        "Database slug,Type\ntest.sqlite,sqlite",
    )?;
    database_details(
        config_path,
        &api_keys.get("second").unwrap()[0],
        FIRST_ENTITY_DB,
        "Database: e2e-first/test.sqlite\nType: sqlite\nAccess level: ReadOnly",
    )?;
    // Read-only access to e2e-first/test.sqlite doesn't grant access
    // to e2e-first/another.sqlite.
    query(
        config_path,
        &api_keys.get("second").unwrap()[0],
        "SELECT COUNT(*) AS the_count FROM test_table;",
        FIRST_ENTITY_DB2,
        "table",
        "Error: Authenticated entity e2e-second can\'t query database e2e-first/another.sqlite",
    )?;
    database_details(
        config_path,
        &api_keys.get("second").unwrap()[0],
        FIRST_ENTITY_DB2,
        "Error: Authenticated entity e2e-second can't access database e2e-first/another.sqlite",
    )?;

    // With no public permissions, the second entity can't query or discover the database.
    update_database(
        config_path,
        &api_keys.get("first").unwrap()[0],
        FIRST_ENTITY_DB,
        "no-access",
        "Database e2e-first/test.sqlite updated successfully",
    )?;
    query(
        config_path,
        &api_keys.get("second").unwrap()[0],
        "SELECT COUNT(*) AS the_count FROM test_table;",
        FIRST_ENTITY_DB,
        "table",
        "Error: Authenticated entity e2e-second can't query database e2e-first/test.sqlite",
    )?;
    list_databases(
        config_path,
        &api_keys.get("second").unwrap()[0],
        FIRST_ENTITY_SLUG,
        "csv",
        &format!("No queryable databases owned by {}", FIRST_ENTITY_SLUG),
    )?;
    database_details(
        config_path,
        &api_keys.get("second").unwrap()[0],
        FIRST_ENTITY_DB,
        "Error: Authenticated entity e2e-second can't access database e2e-first/test.sqlite",
    )?;

    // The second set of tests cover an entity's individual
    // database-level permissions (no-access, read-only, read-write,
    // manager).

    // Ensure we can't update permissions for owner, even if we're ourselves.
    share(
        config_path,
        &api_keys.get("first").unwrap()[0],
        FIRST_ENTITY_DB,
        FIRST_ENTITY_SLUG,
        "no-access",
        "Error: e2e-first owns e2e-first/test.sqlite, so their permissions can't be changed",
    )?;

    // First entity grants second entity read-only access.
    share(
        config_path,
        &api_keys.get("first").unwrap()[0],
        FIRST_ENTITY_DB,
        SECOND_ENTITY_SLUG,
        "read-only",
        "Permissions for e2e-second on e2e-first/test.sqlite updated successfully",
    )?;
    // Verify that list_database_permissions shows the granted permission.
    list_database_permissions(
        config_path,
        &api_keys.get("first").unwrap()[0],
        FIRST_ENTITY_DB,
        "csv",
        "Entity,Sharing level\ne2e-second,read-only",
    )?;
    // Second entity has read-only access.
    query(
        config_path,
        &api_keys.get("second").unwrap()[0],
        "SELECT COUNT(*) AS the_count FROM test_table;",
        FIRST_ENTITY_DB,
        "table",
        " the_count \n-----------\n 4 \n\nRows: 1",
    )?;
    // Second entity can't modify database.
    query(
        config_path,
        &api_keys.get("second").unwrap()[0],
        "INSERT INTO test_table (fname, lname) VALUES (\"first permissions2\", \"last permissions2\");",
        FIRST_ENTITY_DB,
        "table",
        "Error: Attempted to write to database while in read-only mode",
    )?;
    // Second entity can discover database.
    list_databases(
        config_path,
        &api_keys.get("second").unwrap()[0],
        FIRST_ENTITY_SLUG,
        "csv",
        "Database slug,Type\ntest.sqlite,sqlite",
    )?;
    // Second entity can get details on its ReadOnly access level.
    database_details(
        config_path,
        &api_keys.get("second").unwrap()[0],
        FIRST_ENTITY_DB,
        "Database: e2e-first/test.sqlite\nType: sqlite\nAccess level: ReadOnly",
    )?;
    // Second entity can't manage snapshots on the database.
    list_snapshots_match_output(
        config_path,
        &api_keys.get("second").unwrap()[0],
        FIRST_ENTITY_DB,
        "csv",
        "Error: Authenticated entity e2e-second can't manage snapshots on database e2e-first/test.sqlite",
    )?;
    // Second entity can't update the database's metadata.
    update_database(
        config_path,
        &api_keys.get("second").unwrap()[0],
        FIRST_ENTITY_DB,
        "fork",
        "Error: Authenticated entity e2e-second can't update database e2e-first/test.sqlite",
    )?;
    // Second entity can't share the database with anyone else.
    share(
        config_path,
        &api_keys.get("second").unwrap()[0],
        FIRST_ENTITY_DB,
        THIRD_ENTITY_SLUG,
        "read-only",
        "Error: Authenticated entity e2e-second can\'t set permissions for database e2e-first/test.sqlite",
    )?;
    // Third entity has no access (only the second entity got access).
    query(
        config_path,
        &api_keys.get("third").unwrap()[0],
        "SELECT COUNT(*) AS the_count FROM test_table;",
        FIRST_ENTITY_DB,
        "table",
        "Error: Authenticated entity e2e-third can't query database e2e-first/test.sqlite",
    )?;
    list_databases(
        config_path,
        &api_keys.get("third").unwrap()[0],
        FIRST_ENTITY_SLUG,
        "csv",
        &format!("No queryable databases owned by {}", FIRST_ENTITY_SLUG),
    )?;
    database_details(
        config_path,
        &api_keys.get("third").unwrap()[0],
        FIRST_ENTITY_DB,
        "Error: Authenticated entity e2e-third can't access database e2e-first/test.sqlite",
    )?;

    // First entity upgrades second entity's access to read-write.
    share(
        config_path,
        &api_keys.get("first").unwrap()[0],
        FIRST_ENTITY_DB,
        SECOND_ENTITY_SLUG,
        "read-write",
        "Permissions for e2e-second on e2e-first/test.sqlite updated successfully",
    )?;
    // Second entity can query database.
    query(
        config_path,
        &api_keys.get("second").unwrap()[0],
        "SELECT COUNT(*) AS the_count FROM test_table;",
        FIRST_ENTITY_DB,
        "table",
        " the_count \n-----------\n 4 \n\nRows: 1",
    )?;
    // Even if the public sharing level of the database is read-only,
    // the second entity will be able to modify the database.
    update_database(
        config_path,
        &api_keys.get("first").unwrap()[0],
        FIRST_ENTITY_DB,
        "read-only",
        "Database e2e-first/test.sqlite updated successfully",
    )?;
    query(
        config_path,
        &api_keys.get("second").unwrap()[0],
        "INSERT INTO test_table (fname, lname) VALUES (\"first permissions2\", \"last permissions2\");",
        FIRST_ENTITY_DB,
        "table",
        "\nRows: 0",
    )?;
    update_database(
        config_path,
        &api_keys.get("first").unwrap()[0],
        FIRST_ENTITY_DB,
        "no-access",
        "Database e2e-first/test.sqlite updated successfully",
    )?;
    // Second entity can discover database.
    list_databases(
        config_path,
        &api_keys.get("second").unwrap()[0],
        FIRST_ENTITY_SLUG,
        "csv",
        "Database slug,Type\ntest.sqlite,sqlite",
    )?;
    // Second entity can read its own ReadWrite access.
    database_details(
        config_path,
        &api_keys.get("second").unwrap()[0],
        FIRST_ENTITY_DB,
        "Database: e2e-first/test.sqlite\nType: sqlite\nAccess level: ReadWrite",
    )?;
    // Second entity can't manage snapshots on the database.
    list_snapshots_match_output(
        config_path,
        &api_keys.get("second").unwrap()[0],
        FIRST_ENTITY_DB,
        "csv",
        "Error: Authenticated entity e2e-second can't manage snapshots on database e2e-first/test.sqlite",
    )?;
    // Second entity can't update database metadata.
    update_database(
        config_path,
        &api_keys.get("second").unwrap()[0],
        FIRST_ENTITY_DB,
        "fork",
        "Error: Authenticated entity e2e-second can't update database e2e-first/test.sqlite",
    )?;
    // Second entity can't share the database with anyone else.
    share(
        config_path,
        &api_keys.get("second").unwrap()[0],
        FIRST_ENTITY_DB,
        THIRD_ENTITY_SLUG,
        "read-only",
        "Error: Authenticated entity e2e-second can\'t set permissions for database e2e-first/test.sqlite",
    )?;
    // Third entity has no access.
    query(
        config_path,
        &api_keys.get("third").unwrap()[0],
        "SELECT COUNT(*) AS the_count FROM test_table;",
        FIRST_ENTITY_DB,
        "table",
        "Error: Authenticated entity e2e-third can't query database e2e-first/test.sqlite",
    )?;
    list_databases(
        config_path,
        &api_keys.get("third").unwrap()[0],
        FIRST_ENTITY_SLUG,
        "csv",
        &format!("No queryable databases owned by {}", FIRST_ENTITY_SLUG),
    )?;
    database_details(
        config_path,
        &api_keys.get("third").unwrap()[0],
        FIRST_ENTITY_DB,
        "Error: Authenticated entity e2e-third can't access database e2e-first/test.sqlite",
    )?;

    // First entity updates second entity to manager access.
    share(
        config_path,
        &api_keys.get("first").unwrap()[0],
        FIRST_ENTITY_DB,
        SECOND_ENTITY_SLUG,
        "manager",
        "Permissions for e2e-second on e2e-first/test.sqlite updated successfully",
    )?;
    // Second entity can query database.
    query(
        config_path,
        &api_keys.get("second").unwrap()[0],
        "SELECT COUNT(*) AS the_count FROM test_table;",
        FIRST_ENTITY_DB,
        "table",
        " the_count \n-----------\n 5 \n\nRows: 1",
    )?;
    // Second entity can modify database.
    query(
        config_path,
        &api_keys.get("second").unwrap()[0],
        "INSERT INTO test_table (fname, lname) VALUES (\"first permissions2\", \"last permissions2\");",
        FIRST_ENTITY_DB,
        "table",
        "\nRows: 0",
    )?;
    // Second entity can discover database.
    list_databases(
        config_path,
        &api_keys.get("second").unwrap()[0],
        FIRST_ENTITY_SLUG,
        "csv",
        "Database slug,Type\ntest.sqlite,sqlite",
    )?;
    // Second entity can manage snapshots on the database.
    let snapshots = list_snapshots(
        config_path,
        &api_keys.get("first").unwrap()[0],
        FIRST_ENTITY_DB,
        "csv",
    )?;
    assert_ne!(
        snapshots.len(),
        0,
        "e2e-second should be able to list snapshots"
    );
    // Second entity can get database details and ReadWrite/management access.
    database_details(
        config_path,
        &api_keys.get("second").unwrap()[0],
        FIRST_ENTITY_DB,
        "Database: e2e-first/test.sqlite\nType: sqlite\nAccess level: ReadWrite\nYou have management permissions for this database",
    )?;

    // Access to e2e-first/test.sqlite doesn't grant access to
    // e2e-first/another.sqlite.
    query(
        config_path,
        &api_keys.get("second").unwrap()[0],
        "SELECT COUNT(*) AS the_count FROM test_table;",
        FIRST_ENTITY_DB2,
        "table",
        "Error: Authenticated entity e2e-second can\'t query database e2e-first/another.sqlite",
    )?;
    database_details(
        config_path,
        &api_keys.get("second").unwrap()[0],
        FIRST_ENTITY_DB2,
        "Error: Authenticated entity e2e-second can't access database e2e-first/another.sqlite",
    )?;
    // Second entity can update database metadata.
    update_database(
        config_path,
        &api_keys.get("second").unwrap()[0],
        FIRST_ENTITY_DB,
        "read-only",
        "Database e2e-first/test.sqlite updated successfully",
    )?;
    // Third entity can query database.
    query(
        config_path,
        &api_keys.get("third").unwrap()[0],
        "SELECT COUNT(*) AS the_count FROM test_table;",
        FIRST_ENTITY_DB,
        "table",
        " the_count \n-----------\n 6 \n\nRows: 1",
    )?;
    // Third entity can't modify database.
    query(
        config_path,
        &api_keys.get("third").unwrap()[0],
        "INSERT INTO test_table (fname, lname) VALUES (\"first permissions2\", \"last permissions2\");",
        FIRST_ENTITY_DB,
        "table",
        "Error: Attempted to write to database while in read-only mode",
    )?;
    // Third entity can discover database.
    list_databases(
        config_path,
        &api_keys.get("third").unwrap()[0],
        FIRST_ENTITY_SLUG,
        "csv",
        "Database slug,Type\ntest.sqlite,sqlite",
    )?;
    database_details(
        config_path,
        &api_keys.get("third").unwrap()[0],
        FIRST_ENTITY_DB,
        "Database: e2e-first/test.sqlite\nType: sqlite\nAccess level: ReadOnly",
    )?;
    // Second entity revokes public sharing access.
    update_database(
        config_path,
        &api_keys.get("second").unwrap()[0],
        FIRST_ENTITY_DB,
        "no-access",
        "Database e2e-first/test.sqlite updated successfully",
    )?;
    // Third entity is back to having no access.
    query(
        config_path,
        &api_keys.get("third").unwrap()[0],
        "SELECT COUNT(*) AS the_count FROM test_table;",
        FIRST_ENTITY_DB,
        "table",
        "Error: Authenticated entity e2e-third can't query database e2e-first/test.sqlite",
    )?;
    list_databases(
        config_path,
        &api_keys.get("third").unwrap()[0],
        FIRST_ENTITY_SLUG,
        "csv",
        &format!("No queryable databases owned by {}", FIRST_ENTITY_SLUG),
    )?;
    database_details(
        config_path,
        &api_keys.get("third").unwrap()[0],
        FIRST_ENTITY_DB,
        "Error: Authenticated entity e2e-third can't access database e2e-first/test.sqlite",
    )?;

    // Second entity can share the database with third entity.
    share(
        config_path,
        &api_keys.get("second").unwrap()[0],
        FIRST_ENTITY_DB,
        THIRD_ENTITY_SLUG,
        "read-only",
        "Permissions for e2e-third on e2e-first/test.sqlite updated successfully",
    )?;
    // Verify that list_database_permissions shows both entities with permissions.
    list_database_permissions(
        config_path,
        &api_keys.get("first").unwrap()[0],
        FIRST_ENTITY_DB,
        "csv",
        "Entity,Sharing level\ne2e-second,manager\ne2e-third,read-only",
    )?;
    // Third entity can query database.
    query(
        config_path,
        &api_keys.get("third").unwrap()[0],
        "SELECT COUNT(*) AS the_count FROM test_table;",
        FIRST_ENTITY_DB,
        "table",
        " the_count \n-----------\n 6 \n\nRows: 1",
    )?;
    // Third entity can't modify database.
    query(
        config_path,
        &api_keys.get("third").unwrap()[0],
        "INSERT INTO test_table (fname, lname) VALUES (\"first permissions2\", \"last permissions2\");",
        FIRST_ENTITY_DB,
        "table",
        "Error: Attempted to write to database while in read-only mode",
    )?;
    // Third entity can discover database.
    list_databases(
        config_path,
        &api_keys.get("third").unwrap()[0],
        FIRST_ENTITY_SLUG,
        "csv",
        "Database slug,Type\ntest.sqlite,sqlite",
    )?;
    // Third entity can identify its own ReadOnly access.
    database_details(
        config_path,
        &api_keys.get("third").unwrap()[0],
        FIRST_ENTITY_DB,
        "Database: e2e-first/test.sqlite\nType: sqlite\nAccess level: ReadOnly",
    )?;

    // Second entity revokes third entity's access.
    share(
        config_path,
        &api_keys.get("second").unwrap()[0],
        FIRST_ENTITY_DB,
        THIRD_ENTITY_SLUG,
        "no-access",
        "Permissions for e2e-third on e2e-first/test.sqlite updated successfully",
    )?;
    // Third entity is back to having no access.
    query(
        config_path,
        &api_keys.get("third").unwrap()[0],
        "SELECT COUNT(*) AS the_count FROM test_table;",
        FIRST_ENTITY_DB,
        "table",
        "Error: Authenticated entity e2e-third can't query database e2e-first/test.sqlite",
    )?;
    list_databases(
        config_path,
        &api_keys.get("third").unwrap()[0],
        FIRST_ENTITY_SLUG,
        "csv",
        &format!("No queryable databases owned by {}", FIRST_ENTITY_SLUG),
    )?;
    database_details(
        config_path,
        &api_keys.get("third").unwrap()[0],
        FIRST_ENTITY_DB,
        "Error: Authenticated entity e2e-third can't access database e2e-first/test.sqlite",
    )?;

    // A manager (second entity) can't modify the owner's (first entity's) access.
    share(
        config_path,
        &api_keys.get("second").unwrap()[0],
        FIRST_ENTITY_DB,
        FIRST_ENTITY_SLUG,
        "no-access",
        "Error: e2e-first owns e2e-first/test.sqlite, so their permissions can't be changed",
    )?;

    // First entity revoke's second entity's access.
    share(
        config_path,
        &api_keys.get("first").unwrap()[0],
        FIRST_ENTITY_DB,
        SECOND_ENTITY_SLUG,
        "no-access",
        "Permissions for e2e-second on e2e-first/test.sqlite updated successfully",
    )?;
    // Verify that list_database_permissions shows no shared permissions after all are revoked.
    list_database_permissions(
        config_path,
        &api_keys.get("first").unwrap()[0],
        FIRST_ENTITY_DB,
        "csv",
        "No shared permissions for e2e-first/test.sqlite",
    )?;
    // Second entity now has no access.
    query(
        config_path,
        &api_keys.get("second").unwrap()[0],
        "SELECT COUNT(*) AS the_count FROM test_table;",
        FIRST_ENTITY_DB,
        "table",
        "Error: Authenticated entity e2e-second can't query database e2e-first/test.sqlite",
    )?;
    list_databases(
        config_path,
        &api_keys.get("second").unwrap()[0],
        FIRST_ENTITY_SLUG,
        "csv",
        &format!("No queryable databases owned by {}", FIRST_ENTITY_SLUG),
    )?;
    database_details(
        config_path,
        &api_keys.get("second").unwrap()[0],
        FIRST_ENTITY_DB,
        "Error: Authenticated entity e2e-second can't access database e2e-first/test.sqlite",
    )?;

    Ok(())
}
