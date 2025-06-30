use crate::e2e_tests::{
    FIRST_ENTITY_DB, FIRST_ENTITY_DB_CASED, FIRST_ENTITY_DB_SLUG, FIRST_ENTITY_SLUG,
};
use crate::utils::ayb::{list_snapshots, list_snapshots_match_output, query, restore_snapshot};
use crate::utils::testing::snapshot_storage;
use std::collections::HashMap;
use std::thread;
use std::time::{Duration, Instant};

pub async fn test_snapshots(
    db_type: &str,
    config_path: &str,
    api_keys: &HashMap<String, Vec<String>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let test_start = Instant::now();
    println!(
        "[SNAPSHOT_TEST] Starting snapshot tests for db_type: {}, config: {}",
        db_type, config_path
    );
    println!(
        "[SNAPSHOT_TEST] Test constants - entity: {}, db: {}, db_slug: {}",
        FIRST_ENTITY_SLUG, FIRST_ENTITY_DB, FIRST_ENTITY_DB_SLUG
    );

    // Log API keys (without revealing the actual keys)
    println!(
        "[SNAPSHOT_TEST] API keys available: {:?}",
        api_keys
            .keys()
            .map(|k| format!("{} (count: {})", k, api_keys.get(k).unwrap().len()))
            .collect::<Vec<_>>()
    );

    // Can't list snapshots from an account without access.
    println!("[SNAPSHOT_TEST] Step 1: Testing unauthorized snapshot access");
    list_snapshots_match_output(
        config_path,
        &api_keys.get("second").unwrap()[0],
        FIRST_ENTITY_DB,
        "csv",
        "Error: Authenticated entity e2e-second can't manage snapshots on database e2e-first/test.sqlite",
    )?;
    println!("[SNAPSHOT_TEST] Step 1: Unauthorized access test passed");

    // Remove all snapshots so our tests aren't affected by
    // timing/snapshots from previous tests.
    println!("[SNAPSHOT_TEST] Step 2: Cleaning up existing snapshots");
    let storage_start = Instant::now();
    let storage = snapshot_storage(db_type).await?;
    println!(
        "[SNAPSHOT_TEST] Storage connection established in {:?}",
        storage_start.elapsed()
    );

    let list_start = Instant::now();
    let existing_snapshots = storage
        .list_snapshots(FIRST_ENTITY_SLUG, FIRST_ENTITY_DB_SLUG)
        .await?;
    println!(
        "[SNAPSHOT_TEST] Listed {} existing snapshots in {:?}",
        existing_snapshots.len(),
        list_start.elapsed()
    );

    if !existing_snapshots.is_empty() {
        println!(
            "[SNAPSHOT_TEST] Existing snapshots to delete: {:?}",
            existing_snapshots
                .iter()
                .map(|s| &s.snapshot_id)
                .collect::<Vec<_>>()
        );

        let delete_start = Instant::now();
        storage
            .delete_snapshots(
                FIRST_ENTITY_SLUG,
                FIRST_ENTITY_DB_SLUG,
                &existing_snapshots
                    .iter()
                    .map(|snapshot| snapshot.snapshot_id.clone())
                    .collect(),
            )
            .await?;
        println!(
            "[SNAPSHOT_TEST] Deleted {} snapshots in {:?}",
            existing_snapshots.len(),
            delete_start.elapsed()
        );
    } else {
        println!("[SNAPSHOT_TEST] No existing snapshots to clean up");
    }

    // Can list snapshots from the first set of API keys.
    println!("[SNAPSHOT_TEST] Step 3: Testing authorized empty snapshot list");
    list_snapshots_match_output(
        config_path,
        &api_keys.get("first").unwrap()[0],
        FIRST_ENTITY_DB_CASED,
        "csv",
        "No snapshots for E2E-FiRST/test.sqlite",
    )?;
    println!("[SNAPSHOT_TEST] Step 3: Empty snapshot list test passed");

    // We'll sleep between various checks in this test to allow the
    // snapshotting logic, which runs every 2 seconds, to execute.
    println!("[SNAPSHOT_TEST] Step 4: Waiting for automatic snapshot creation (4 second sleep)");
    let sleep_start = Instant::now();
    thread::sleep(Duration::from_secs(4));
    println!(
        "[SNAPSHOT_TEST] Sleep completed in {:?}",
        sleep_start.elapsed()
    );

    println!("[SNAPSHOT_TEST] Step 4: Checking for first automatic snapshot");
    let list_start = Instant::now();
    let snapshots = list_snapshots(
        config_path,
        &api_keys.get("first").unwrap()[0],
        FIRST_ENTITY_DB,
        "csv",
    )?;
    println!(
        "[SNAPSHOT_TEST] Listed {} snapshots in {:?}",
        snapshots.len(),
        list_start.elapsed()
    );

    if snapshots.len() > 0 {
        println!(
            "[SNAPSHOT_TEST] Found snapshots: {:?}",
            snapshots
                .iter()
                .map(|s| format!("{} ({})", s.snapshot_id, s.last_modified_at))
                .collect::<Vec<_>>()
        );
    }

    let last_modified_at = snapshots[0].last_modified_at;
    assert_eq!(
        snapshots.len(),
        1,
        "[SNAPSHOT_TEST] there should be one snapshot after sleeping, found: {}",
        snapshots.len()
    );
    println!(
        "[SNAPSHOT_TEST] Step 4: Found expected 1 snapshot, last_modified: {}",
        last_modified_at
    );

    // No change to database, so same number of snapshots after sleep.
    println!("[SNAPSHOT_TEST] Step 5: Testing snapshot stability (no DB change, 4 second sleep)");
    let sleep_start = Instant::now();
    thread::sleep(Duration::from_secs(4));
    println!(
        "[SNAPSHOT_TEST] Sleep completed in {:?}",
        sleep_start.elapsed()
    );

    let list_start = Instant::now();
    let snapshots = list_snapshots(
        config_path,
        &api_keys.get("first").unwrap()[0],
        FIRST_ENTITY_DB,
        "csv",
    )?;
    println!(
        "[SNAPSHOT_TEST] Listed {} snapshots in {:?}",
        snapshots.len(),
        list_start.elapsed()
    );

    assert_eq!(
        snapshots.len(),
        1,
        "[SNAPSHOT_TEST] there should still be one snapshot after sleeping more, found: {}",
        snapshots.len()
    );
    assert_eq!(
        last_modified_at, snapshots[0].last_modified_at,
        "[SNAPSHOT_TEST] After sleeping, the snapshot shouldn't have been modified/updated. Expected: {}, Found: {}", 
        last_modified_at, snapshots[0].last_modified_at
    );
    println!("[SNAPSHOT_TEST] Step 5: Snapshot stability confirmed - no new snapshots created");

    // Modify database, wait, and ensure a new snapshot was taken.
    println!("[SNAPSHOT_TEST] Step 6: Modifying database and waiting for new snapshot");
    let query_start = Instant::now();
    query(
        config_path,
        &api_keys.get("first").unwrap()[1],
        "INSERT INTO test_table (fname, lname) VALUES (\"another first\", \"another last\");",
        FIRST_ENTITY_DB,
        "table",
        "\nRows: 0",
    )?;
    println!(
        "[SNAPSHOT_TEST] Database modification completed in {:?}",
        query_start.elapsed()
    );

    println!("[SNAPSHOT_TEST] Step 6: Waiting for snapshot after DB change (4 second sleep)");
    let sleep_start = Instant::now();
    thread::sleep(Duration::from_secs(4));
    println!(
        "[SNAPSHOT_TEST] Sleep completed in {:?}",
        sleep_start.elapsed()
    );

    let list_start = Instant::now();
    let snapshots = list_snapshots(
        config_path,
        &api_keys.get("first").unwrap()[0],
        FIRST_ENTITY_DB,
        "csv",
    )?;
    println!(
        "[SNAPSHOT_TEST] Listed {} snapshots in {:?}",
        snapshots.len(),
        list_start.elapsed()
    );

    if snapshots.len() > 0 {
        println!(
            "[SNAPSHOT_TEST] Current snapshots: {:?}",
            snapshots
                .iter()
                .map(|s| format!("{} ({})", s.snapshot_id, s.last_modified_at))
                .collect::<Vec<_>>()
        );
    }

    assert_eq!(
        snapshots.len(),
        2,
        "[SNAPSHOT_TEST] there should be two snapshots after updating database, found: {}",
        snapshots.len()
    );
    println!("[SNAPSHOT_TEST] Step 6: Found expected 2 snapshots after database modification");

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
    println!(
        "[SNAPSHOT_TEST] Step 7: Testing snapshot restoration to {}",
        snapshots[0].snapshot_id
    );
    let restore_start = Instant::now();
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
    println!(
        "[SNAPSHOT_TEST] Snapshot restoration completed in {:?}",
        restore_start.elapsed()
    );
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

    // There are 3 max_snapshots, so let's
    // force 2 more snapshots to be created (more than 3 snapshots
    // would exist) and then: 1) Ensure there are still only 3
    // snapshots remaining due to pruning, 2) Get an error restoring
    // to the oldest snapshot, which should have been pruned.
    println!("[SNAPSHOT_TEST] Step 8: Testing snapshot pruning (max_snapshots=3)");
    println!("[SNAPSHOT_TEST] Step 8a: Adding first row to trigger new snapshot");
    query(
        config_path,
        &api_keys.get("first").unwrap()[1],
        "INSERT INTO test_table (fname, lname) VALUES (\"a new first name\", \"a new last name\");",
        FIRST_ENTITY_DB,
        "table",
        "\nRows: 0",
    )?;
    println!("[SNAPSHOT_TEST] Step 8a: Waiting for snapshot creation (4 second sleep)");
    let sleep_start = Instant::now();
    thread::sleep(Duration::from_secs(4));
    println!(
        "[SNAPSHOT_TEST] Sleep completed in {:?}",
        sleep_start.elapsed()
    );

    println!("[SNAPSHOT_TEST] Step 8b: Adding second row to trigger another snapshot");
    query(
        config_path,
        &api_keys.get("first").unwrap()[1],
        "INSERT INTO test_table (fname, lname) VALUES (\"and another new first name\", \"and another new last name\");",
        FIRST_ENTITY_DB,
        "table",
        "\nRows: 0",
    )?;
    println!("[SNAPSHOT_TEST] Step 8b: Waiting for snapshot creation and pruning (4 second sleep)");
    let sleep_start = Instant::now();
    thread::sleep(Duration::from_secs(4));
    println!(
        "[SNAPSHOT_TEST] Sleep completed in {:?}",
        sleep_start.elapsed()
    );

    let old_snapshots = snapshots;
    println!("[SNAPSHOT_TEST] Step 8c: Checking snapshot count after pruning");
    let list_start = Instant::now();
    let snapshots = list_snapshots(
        config_path,
        &api_keys.get("first").unwrap()[0],
        FIRST_ENTITY_DB,
        "csv",
    )?;
    println!(
        "[SNAPSHOT_TEST] Listed {} snapshots in {:?}",
        snapshots.len(),
        list_start.elapsed()
    );

    if snapshots.len() > 0 {
        println!(
            "[SNAPSHOT_TEST] Current snapshots after pruning: {:?}",
            snapshots
                .iter()
                .map(|s| format!("{} ({})", s.snapshot_id, s.last_modified_at))
                .collect::<Vec<_>>()
        );
    }

    println!(
        "[SNAPSHOT_TEST] Old snapshots (before pruning): {:?}",
        old_snapshots
            .iter()
            .map(|s| format!("{} ({})", s.snapshot_id, s.last_modified_at))
            .collect::<Vec<_>>()
    );

    assert_eq!(
        snapshots.len(),
        3,
        "[SNAPSHOT_TEST] there should be three snapshots after further updating database and pruning old snapshots, found: {}", snapshots.len()
    );
    println!("[SNAPSHOT_TEST] Step 8: Pruning test passed - found expected 3 snapshots");

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

    println!(
        "[SNAPSHOT_TEST] All snapshot tests completed successfully in {:?}",
        test_start.elapsed()
    );
    Ok(())
}
