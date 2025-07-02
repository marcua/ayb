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
    // Can't list snapshots from an account without access.
    list_snapshots_match_output(
        config_path,
        &api_keys.get("second").unwrap()[0],
        FIRST_ENTITY_DB,
        "csv",
        "Error: Authenticated entity e2e-second can't manage snapshots on database e2e-first/test.sqlite",
    )?;

    // Remove all snapshots so our tests aren't affected by
    // timing/snapshots from previous tests.
    let storage = snapshot_storage(db_type).await?;
    storage
        .delete_snapshots(
            FIRST_ENTITY_SLUG,
            FIRST_ENTITY_DB_SLUG,
            &storage
                .list_snapshots(FIRST_ENTITY_SLUG, FIRST_ENTITY_DB_SLUG)
                .await?
                .iter()
                .map(|snapshot| snapshot.snapshot_id.clone())
                .collect(),
        )
        .await?;

    // Can list snapshots from the first set of API keys.
    list_snapshots_match_output(
        config_path,
        &api_keys.get("first").unwrap()[0],
        FIRST_ENTITY_DB_CASED,
        "csv",
        "No snapshots for E2E-FiRST/test.sqlite",
    )?;
    // We'll sleep between various checks in this test to allow the
    // snapshotting logic, which runs every 2 seconds, to execute.
    thread::sleep(time::Duration::from_secs(4));
    let snapshots = list_snapshots(
        config_path,
        &api_keys.get("first").unwrap()[0],
        FIRST_ENTITY_DB,
        "csv",
    )?;

    let last_modified_at = snapshots[0].last_modified_at;
    assert_eq!(
        snapshots.len(),
        1,
        "there should be one snapshot after sleeping"
    );
    // No change to database, so same number of snapshots after sleep.
    thread::sleep(time::Duration::from_secs(4));
    let snapshots = list_snapshots(
        config_path,
        &api_keys.get("first").unwrap()[0],
        FIRST_ENTITY_DB,
        "csv",
    )?;
    assert_eq!(
        snapshots.len(),
        1,
        "there should still be one snapshot after sleeping more"
    );
    assert_eq!(
        last_modified_at, snapshots[0].last_modified_at,
        "After sleeping, the snapshot shouldn't have been modified/updated"
    );
    // Modify database, wait, and ensure a new snapshot was taken.
    query(
        config_path,
        &api_keys.get("first").unwrap()[1],
        "INSERT INTO test_table (fname, lname) VALUES (\"another first\", \"another last\");",
        FIRST_ENTITY_DB,
        "table",
        "\nRows: 0",
    )?;
    thread::sleep(time::Duration::from_secs(4));
    let snapshots = list_snapshots(
        config_path,
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
        config_path,
        &api_keys.get("first").unwrap()[1],
        "INSERT INTO test_table (fname, lname) VALUES (\"yet another first\", \"yet another last\");",
        FIRST_ENTITY_DB,
        "table",
        "\nRows: 0",
    )?;
    query(
        config_path,
        &api_keys.get("first").unwrap()[1],
        "SELECT COUNT(*) AS the_count FROM test_table;",
        FIRST_ENTITY_DB,
        "table",
        " the_count \n-----------\n 4 \n\nRows: 1",
    )?;

    // Restore the previous snapshot, ensuring there are only three
    // rows.
    restore_snapshot(
        config_path,
        &api_keys.get("first").unwrap()[0],
        FIRST_ENTITY_DB,
        &snapshots[0].snapshot_id,
        &format!(
            "Restored e2e-first/test.sqlite to snapshot {}",
            snapshots[0].snapshot_id
        ),
    )?;
    query(
        config_path,
        &api_keys.get("first").unwrap()[1],
        "SELECT COUNT(*) AS the_count FROM test_table;",
        FIRST_ENTITY_DB,
        "table",
        " the_count \n-----------\n 3 \n\nRows: 1",
    )?;

    // Restore the snapshot before that, ensuring there are only two
    // rows.
    restore_snapshot(
        config_path,
        &api_keys.get("first").unwrap()[0],
        FIRST_ENTITY_DB,
        &snapshots[1].snapshot_id,
        &format!(
            "Restored e2e-first/test.sqlite to snapshot {}",
            snapshots[1].snapshot_id
        ),
    )?;
    query(
        config_path,
        &api_keys.get("first").unwrap()[1],
        "SELECT COUNT(*) AS the_count FROM test_table;",
        FIRST_ENTITY_DB,
        "table",
        " the_count \n-----------\n 2 \n\nRows: 1",
    )?;

    // Restoring a snapshot causes another snapshot to be taken (the
    // contents of the database are logically equivalent but
    // physically different). Note that there's a theoretical race
    // condition here in case a snapshot is taken between the two
    // restores above. Make tests less brittle if it ever arises.
    thread::sleep(time::Duration::from_secs(4));

    // There are 4 max_snapshots, so let's force 2 more snapshots to
    // be created (more than 4 snapshots would exist: the original
    // two, one after the restore, and two more from the inserts
    // below) and then: 1) Ensure there are still only 4 snapshots
    // remaining due to pruning, 2) Get an error restoring to the
    // oldest snapshot, which should have been pruned.
    query(
        config_path,
        &api_keys.get("first").unwrap()[1],
        "INSERT INTO test_table (fname, lname) VALUES (\"a new first name\", \"a new last name\");",
        FIRST_ENTITY_DB,
        "table",
        "\nRows: 0",
    )?;
    thread::sleep(time::Duration::from_secs(4));
    query(
        config_path,
        &api_keys.get("first").unwrap()[1],
        "INSERT INTO test_table (fname, lname) VALUES (\"and another new first name\", \"and another new last name\");",
        FIRST_ENTITY_DB,
        "table",
        "\nRows: 0",
    )?;
    thread::sleep(time::Duration::from_secs(4));
    let old_snapshots = snapshots;
    let snapshots = list_snapshots(
        config_path,
        &api_keys.get("first").unwrap()[0],
        FIRST_ENTITY_DB,
        "csv",
    )?;
    assert_eq!(
        snapshots.len(),
        4,
        "there are four snapshots after further updating database and pruning old snapshots"
    );

    // Restoring the previous oldest snapshot fails
    restore_snapshot(
        config_path,
        &api_keys.get("first").unwrap()[0],
        FIRST_ENTITY_DB,
        &old_snapshots[1].snapshot_id,
        &format!(
            "Error: Snapshot {} does not exist for e2e-first/test.sqlite",
            &old_snapshots[1].snapshot_id
        ),
    )?;
    query(
        config_path,
        &api_keys.get("first").unwrap()[1],
        "SELECT COUNT(*) AS the_count FROM test_table WHERE fname = \"and another new first name\";",
        FIRST_ENTITY_DB,
        "table",
        " the_count \n-----------\n 1 \n\nRows: 1",
    )?;

    // Restoring the newer of the two oldest snapshot succeeds
    restore_snapshot(
        config_path,
        &api_keys.get("first").unwrap()[0],
        FIRST_ENTITY_DB,
        &old_snapshots[0].snapshot_id,
        &format!(
            "Restored e2e-first/test.sqlite to snapshot {}",
            old_snapshots[0].snapshot_id
        ),
    )?;
    query(
        config_path,
        &api_keys.get("first").unwrap()[1],
        "SELECT COUNT(*) AS the_count FROM test_table WHERE fname = \"and another new first name\";",
        FIRST_ENTITY_DB,
        "table",
        " the_count \n-----------\n 0 \n\nRows: 1",
    )?;

    Ok(())
}
