use crate::e2e_tests::{
    FIRST_ENTITY_DB, FIRST_ENTITY_DB_CASED, FIRST_ENTITY_DB_SLUG, FIRST_ENTITY_SLUG,
};
use crate::utils::ayb::{list_snapshots, query};
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
    list_snapshots(
        &config_path,
        &api_keys.get("second").unwrap()[0],
        FIRST_ENTITY_DB,
        "csv",
        "Error: Authenticated entity e2e-second can not manage snapshots on database e2e-first/test.sqlite",
    )?;

    // Can list snapshots from the first set of API keys.
    list_snapshots(
        &config_path,
        &api_keys.get("first").unwrap()[0],
        FIRST_ENTITY_DB_CASED,
        "csv",
        "No snapshots for E2E-FiRST/test.sqlite",
    )?;

    // We'll sleep between various checks in this test to allow the
    // snapshotting logic, which runs every 2 seconds, to execute.
    let snapshot_result_line = format!(
        r"bucket\/{}\/e2e-first\/test.sqlite\/notimplemented,\d{{4,5}}-\d{{2}}-\d{{2}} \d{{2}}:\d{{2}}:\d{{2}} UTC",
        db_type
    );
    thread::sleep(time::Duration::from_secs(3));
    list_snapshots(
        &config_path,
        &api_keys.get("first").unwrap()[0],
        FIRST_ENTITY_DB,
        "csv",
        &format!("Name,Last modified\n{}", snapshot_result_line),
    )?;

    // No change to database, so same number of snapshots after sleep.
    thread::sleep(time::Duration::from_secs(3));
    list_snapshots(
        &config_path,
        &api_keys.get("first").unwrap()[0],
        FIRST_ENTITY_DB,
        "csv",
        &format!("Name,Last modified\n{}", snapshot_result_line),
    )?;

    // Modify database, wait, and ensure a new snapshot was taken.
    query(
        &config_path,
        &api_keys.get("first").unwrap()[1],
        "INSERT INTO test_table (fname, lname) VALUES (\"another first\", \"another last\");",
        FIRST_ENTITY_DB,
        "table",
        "\nRows: 0",
    )?;
    thread::sleep(time::Duration::from_secs(3));
    // TODO(marcua): When we store multiple snapshots with hashing,
    // there should be two snapshots instead of one here.
    list_snapshots(
        &config_path,
        &api_keys.get("first").unwrap()[0],
        FIRST_ENTITY_DB,
        "csv",
        &format!("Name,Last modified\n{}", snapshot_result_line),
    )?;

    Ok(())
}
