use crate::e2e_tests::{
    FIRST_ENTITY_DB, FIRST_ENTITY_DB_SLUG, FIRST_ENTITY_DUCKDB, FIRST_ENTITY_DUCKDB_SLUG,
    FIRST_ENTITY_SLUG,
};
use crate::utils::ayb::{list_snapshots, list_snapshots_match_output, query, restore_snapshot};
use crate::utils::testing::snapshot_storage;
use std::collections::HashMap;
use std::thread;
use std::time;

/// Poll until the snapshot list reaches `expected` entries. When
/// `changed_since` is `Some(id)`, also require the newest snapshot's
/// ID to differ from `id` — this handles the pruning case where the
/// count stays the same but the contents change.
fn wait_for_snapshot_count(
    config_path: &str,
    api_key: &str,
    database: &str,
    expected: usize,
    changed_since: Option<&str>,
) -> Vec<ayb::server::snapshots::models::ListSnapshotResult> {
    let timeout_secs = 20;
    let deadline = time::Instant::now() + time::Duration::from_secs(timeout_secs);
    loop {
        thread::sleep(time::Duration::from_secs(2));
        let snapshots = list_snapshots(config_path, api_key, database, "csv")
            .expect("failed to list snapshots");
        let count_ok = snapshots.len() == expected;
        let newest_ok = match changed_since {
            Some(prev_id) => !snapshots.is_empty() && snapshots[0].snapshot_id != prev_id,
            None => true,
        };
        if (count_ok && newest_ok) || time::Instant::now() >= deadline {
            assert_eq!(
                snapshots.len(),
                expected,
                "expected {} snapshots but found {} (after {}s timeout)",
                expected,
                snapshots.len(),
                timeout_secs
            );
            if let Some(prev_id) = changed_since {
                assert!(
                    newest_ok,
                    "newest snapshot did not change from {} within {}s",
                    prev_id, timeout_secs
                );
            }
            return snapshots;
        }
    }
}

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

    // The background snapshot daemon runs every 2 seconds. After
    // deleting all snapshots, the daemon will quickly recreate one
    // for the current database state. Rather than trying to observe
    // the zero-snapshot window (which is a race), we wait for the
    // daemon to produce the first snapshot.
    let snapshots = wait_for_snapshot_count(
        config_path,
        &api_keys.get("first").unwrap()[0],
        FIRST_ENTITY_DB,
        1,
        None,
    );

    // No change to database, so same number of snapshots after sleep.
    let last_modified_at = snapshots[0].last_modified_at;
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
    let snapshots = wait_for_snapshot_count(
        config_path,
        &api_keys.get("first").unwrap()[0],
        FIRST_ENTITY_DB,
        2,
        None,
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

    // Wait for snapshot of the latest insert to be taken before restoring.
    wait_for_snapshot_count(
        config_path,
        &api_keys.get("first").unwrap()[0],
        FIRST_ENTITY_DB,
        3,
        None,
    );

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

    // Wait for the restore-triggered snapshot before doing the next restore.
    wait_for_snapshot_count(
        config_path,
        &api_keys.get("first").unwrap()[0],
        FIRST_ENTITY_DB,
        4,
        None,
    );

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

    // Wait for restore-triggered snapshot.
    wait_for_snapshot_count(
        config_path,
        &api_keys.get("first").unwrap()[0],
        FIRST_ENTITY_DB,
        5,
        None,
    );

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

    // Wait for snapshot of the insert above (6th snapshot, no pruning yet).
    let snapshots_before_prune = wait_for_snapshot_count(
        config_path,
        &api_keys.get("first").unwrap()[0],
        FIRST_ENTITY_DB,
        6,
        None,
    );

    query(
        config_path,
        &api_keys.get("first").unwrap()[1],
        "INSERT INTO test_table (fname, lname) VALUES (\"and another new first name\", \"and another new last name\");",
        FIRST_ENTITY_DB,
        "table",
        "\nRows: 0",
    )?;

    let old_snapshots = snapshots;
    // The 7th snapshot triggers pruning back to 6. The count stays
    // at 6, but the newest snapshot ID changes. Wait for that.
    let snapshots = wait_for_snapshot_count(
        config_path,
        &api_keys.get("first").unwrap()[0],
        FIRST_ENTITY_DB,
        6,
        Some(&snapshots_before_prune[0].snapshot_id),
    );
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

/// A simpler snapshot/restore cycle for a DuckDB database, paralleling
/// the SQLite test above. Assumes `test_create_and_query_duckdb` has
/// already created `e2e-first/test.duckdb` with two rows.
pub async fn test_snapshots_duckdb(
    db_type: &str,
    config_path: &str,
    api_keys: &HashMap<String, Vec<String>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let api_key = &api_keys.get("first").unwrap()[0];

    // Object storage (MinIO) persists across test runs even though the
    // data directory is reset, so clear any DuckDB snapshots left behind
    // by previous runs to get a deterministic starting count.
    let storage = snapshot_storage(db_type).await?;
    storage
        .delete_snapshots(
            FIRST_ENTITY_SLUG,
            FIRST_ENTITY_DUCKDB_SLUG,
            &storage
                .list_snapshots(FIRST_ENTITY_SLUG, FIRST_ENTITY_DUCKDB_SLUG)
                .await?
                .iter()
                .map(|snapshot| snapshot.snapshot_id.clone())
                .collect(),
        )
        .await?;

    // The database has two rows from test_create_and_query_duckdb. Wait
    // for the snapshot daemon to capture that state, and record it.
    let snapshots = wait_for_snapshot_count(config_path, api_key, FIRST_ENTITY_DUCKDB, 1, None);
    let two_row_snapshot = snapshots[0].snapshot_id.clone();
    query(
        config_path,
        api_key,
        "SELECT count(*) AS the_count FROM test_table;",
        FIRST_ENTITY_DUCKDB,
        "table",
        " the_count \n-----------\n 2 \n\nRows: 1",
    )?;

    // Insert a third row (DuckDB returns a one-row affected-count result,
    // so "Rows: 1") and wait for a second, distinct snapshot.
    query(
        config_path,
        api_key,
        "INSERT INTO test_table VALUES ('the third', 'the last3');",
        FIRST_ENTITY_DUCKDB,
        "table",
        "\nRows: 1",
    )?;
    wait_for_snapshot_count(
        config_path,
        api_key,
        FIRST_ENTITY_DUCKDB,
        2,
        Some(&two_row_snapshot),
    );
    query(
        config_path,
        api_key,
        "SELECT count(*) AS the_count FROM test_table;",
        FIRST_ENTITY_DUCKDB,
        "table",
        " the_count \n-----------\n 3 \n\nRows: 1",
    )?;

    // Restore the two-row snapshot and confirm the third row is gone.
    restore_snapshot(
        config_path,
        api_key,
        FIRST_ENTITY_DUCKDB,
        &two_row_snapshot,
        &format!("Restored e2e-first/test.duckdb to snapshot {two_row_snapshot}"),
    )?;
    query(
        config_path,
        api_key,
        "SELECT count(*) AS the_count FROM test_table;",
        FIRST_ENTITY_DUCKDB,
        "table",
        " the_count \n-----------\n 2 \n\nRows: 1",
    )?;

    Ok(())
}
