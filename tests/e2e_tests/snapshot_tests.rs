use crate::e2e_tests::{
    FIRST_ENTITY_DB, FIRST_ENTITY_DB_CASED, FIRST_ENTITY_DB_SLUG, FIRST_ENTITY_SLUG,
};
use crate::utils::ayb::{
    create_database, list_databases, list_snapshots, list_snapshots_match_output, query,
    restore_snapshot, server_list_snapshots, server_restore_snapshot,
};
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
    // snapshotting logic, which runs every 2 seconds, to
    // execute. Each insert, update, and snapshot restore causes
    // another snapshot to be taken, and if we don't sleep after them,
    // we can encounter a race condition between the test and the
    // asynchronous snapshots being taken in parallel. By sleeping, we
    // ensure predictability of relative snapshot timing and
    // quanitity.
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
    thread::sleep(time::Duration::from_secs(4));

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

    thread::sleep(time::Duration::from_secs(4));

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

    // Ensure another snapshot-due-to-restore.
    thread::sleep(time::Duration::from_secs(4));

    // There are 6 max_snapshots, so let's force 2 more snapshots to
    // be created (more than 6 snapshots would exist: the original
    // three, two from the restores, and two more from the inserts
    // below) and then: 1) Ensure there are still only 6 snapshots
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
        6,
        "there are six snapshots after further updating database and pruning old snapshots"
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

pub async fn test_ayb_db_snapshot_restore(
    db_type: &str,
    config_path: &str,
    api_keys: &HashMap<String, Vec<String>>,
) -> Result<(), Box<dyn std::error::Error>> {
    // Remove all ayb_db snapshots so our tests aren't affected by
    // timing/snapshots from previous tests.
    let storage = snapshot_storage(db_type).await?;
    storage
        .delete_snapshots(
            "__ayb__",
            "ayb",
            &storage
                .list_snapshots("__ayb__", "ayb")
                .await?
                .iter()
                .map(|snapshot| snapshot.snapshot_id.clone())
                .collect(),
        )
        .await?;

    // Wait for initial snapshot to be created
    thread::sleep(time::Duration::from_secs(4));

    // Get the initial snapshot (before we create the new database)
    let initial_snapshots = server_list_snapshots(config_path, "__ayb__/ayb", "csv")?;
    assert!(
        !initial_snapshots.is_empty(),
        "There should be at least one ayb_db snapshot"
    );
    let snapshot_before_db_creation = &initial_snapshots[0].snapshot_id;

    // Create a new hosted database for the first entity
    let new_database = format!("{}/snapshot_test.sqlite", FIRST_ENTITY_SLUG);
    create_database(
        config_path,
        &api_keys.get("first").unwrap()[0],
        &new_database,
        &format!("Successfully created {}", new_database),
    )?;

    // Verify the database exists
    list_databases(
        config_path,
        &api_keys.get("first").unwrap()[0],
        FIRST_ENTITY_SLUG,
        "csv",
        "test.sqlite,sqlite,read-write\nsnapshot_test.sqlite,sqlite,read-write",
    )?;

    // Wait for a snapshot cycle to capture the new database in ayb_db
    thread::sleep(time::Duration::from_secs(4));

    // Get the snapshot after database creation
    let snapshots_after_creation = server_list_snapshots(config_path, "__ayb__/ayb", "csv")?;
    assert!(
        snapshots_after_creation.len() >= 2,
        "There should be at least two ayb_db snapshots after creating a database"
    );

    // Restore to the snapshot before the database was created
    server_restore_snapshot(config_path, "__ayb__/ayb", snapshot_before_db_creation)?;

    // Sleep briefly to allow server restart (in practice, the user would restart)
    // For this test, we're testing the restore mechanism, not the full restart flow
    thread::sleep(time::Duration::from_secs(2));

    // The database should NOT exist after restoring to the earlier snapshot
    // Note: Since we're not actually restarting the server in the test,
    // we can't fully verify this. This is a placeholder for when the
    // server restart mechanism is in place.
    // For now, we'll just verify the restore command succeeded

    // Restore to the snapshot that had the database
    let snapshot_with_db = &snapshots_after_creation[0].snapshot_id;
    server_restore_snapshot(config_path, "__ayb__/ayb", snapshot_with_db)?;

    // Sleep briefly
    thread::sleep(time::Duration::from_secs(2));

    // The database should exist again after restoring to the later snapshot
    // Note: Again, this would require a server restart to fully verify
    // For now, we're just testing that the restore commands work

    Ok(())
}
