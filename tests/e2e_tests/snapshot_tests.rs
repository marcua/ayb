use crate::e2e_tests::{
    FIRST_ENTITY_DB, FIRST_ENTITY_DB_CASED, FIRST_ENTITY_DB_SLUG, FIRST_ENTITY_SLUG,
};
use crate::utils::ayb::{list_snapshots, list_snapshots_match_output, query, restore_snapshot};
use crate::utils::testing::snapshot_storage;
use std::collections::HashMap;
use std::thread;
use std::time;

pub async fn test_snapshots(
    db_type: &str,
    config_path: &str,
    api_keys: &HashMap<String, Vec<String>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let snapshots = snapshot_storage(db_type).await?;
    snapshots
        .dangerously_delete_prefix(FIRST_ENTITY_SLUG, FIRST_ENTITY_DB_SLUG)
        .await?;

    // Can't list snapshots from an account without access.
    list_snapshots_match_output(
        &config_path,
        &api_keys.get("second").unwrap()[0],
        FIRST_ENTITY_DB,
        "csv",
        "Error: Authenticated entity e2e-second can not manage snapshots on database e2e-first/test.sqlite",
    )?;

    // Can list snapshots from the first set of API keys.
    list_snapshots_match_output(
        &config_path,
        &api_keys.get("first").unwrap()[0],
        FIRST_ENTITY_DB_CASED,
        "csv",
        "No snapshots for E2E-FiRST/test.sqlite",
    )?;

    // We'll sleep between various checks in this test to allow the
    // snapshotting logic, which runs every 2 seconds, to execute.
    thread::sleep(time::Duration::from_secs(4));
    let snapshots = list_snapshots(
        &config_path,
        &api_keys.get("first").unwrap()[0],
        FIRST_ENTITY_DB,
        "csv",
    )?;
    assert_eq!(
        snapshots.len(),
        1,
        "there should be one snapshot after sleeping"
    );

    // No change to database, so same number of snapshots after sleep.
    thread::sleep(time::Duration::from_secs(4));
    let snapshots = list_snapshots(
        &config_path,
        &api_keys.get("first").unwrap()[0],
        FIRST_ENTITY_DB,
        "csv",
    )?;
    assert_eq!(
        snapshots.len(),
        1,
        "there should still be one snapshot after sleeping more"
    );

    // Modify database, wait, and ensure a new snapshot was taken.
    query(
        &config_path,
        &api_keys.get("first").unwrap()[1],
        "INSERT INTO test_table (fname, lname) VALUES (\"another first\", \"another last\");",
        FIRST_ENTITY_DB,
        "table",
        "\nRows: 0",
    )?;
    thread::sleep(time::Duration::from_secs(4));
    let snapshots = list_snapshots(
        &config_path,
        &api_keys.get("first").unwrap()[0],
        FIRST_ENTITY_DB,
        "csv",
    )?;
    assert_eq!(
        snapshots.len(),
        2,
        "there two snapshots after updating database"
    );

    // Insert another row and ensure there are four.
    query(
        &config_path,
        &api_keys.get("first").unwrap()[1],
        "INSERT INTO test_table (fname, lname) VALUES (\"yet another first\", \"yet another last\");",
        FIRST_ENTITY_DB,
        "table",
        "\nRows: 0",
    )?;
    query(
        &config_path,
        &api_keys.get("first").unwrap()[1],
        "SELECT COUNT(*) AS the_count FROM test_table;",
        FIRST_ENTITY_DB,
        "table",
        " the_count \n-----------\n 4 \n\nRows: 1",
    )?;

    // Restore the previous snapshot, ensuring there are only three
    // rows.
    restore_snapshot(
        &config_path,
        &api_keys.get("first").unwrap()[0],
        FIRST_ENTITY_DB,
        &snapshots[0].snapshot_id,
        &format!(
            "Restored e2e-first/test.sqlite to snapshot {}",
            snapshots[0].snapshot_id
        ),
    )?;
    query(
        &config_path,
        &api_keys.get("first").unwrap()[1],
        "SELECT COUNT(*) AS the_count FROM test_table;",
        FIRST_ENTITY_DB,
        "table",
        " the_count \n-----------\n 3 \n\nRows: 1",
    )?;

    // Restore the snapshot before that, ensuring there are only two
    // rows.
    restore_snapshot(
        &config_path,
        &api_keys.get("first").unwrap()[0],
        FIRST_ENTITY_DB,
        &snapshots[1].snapshot_id,
        &format!(
            "Restored e2e-first/test.sqlite to snapshot {}",
            snapshots[1].snapshot_id
        ),
    )?;
    query(
        &config_path,
        &api_keys.get("first").unwrap()[1],
        "SELECT COUNT(*) AS the_count FROM test_table;",
        FIRST_ENTITY_DB,
        "table",
        " the_count \n-----------\n 2 \n\nRows: 1",
    )?;

    Ok(())
}
