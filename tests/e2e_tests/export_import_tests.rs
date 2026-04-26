use crate::utils::ayb::{create_database_from_file, export_database, query};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

/// Round-trip a SQLite database through the new export/create-from-file
/// endpoints. This test depends on the databases populated by
/// `test_create_and_query_db`, so it must run after that one.
pub fn test_export_and_import(
    test_type: &str,
    config_path: &str,
    api_keys: &HashMap<String, Vec<String>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let work_dir = PathBuf::from(format!("tests/ayb_data_{test_type}/export_import"));
    fs::create_dir_all(&work_dir)?;
    let exported = work_dir.join("exported.sqlite");
    let exported_str = exported.to_str().unwrap();

    // Owner can export their database. The exported file should be a
    // valid SQLite DB with the rows we previously inserted.
    export_database(
        config_path,
        &api_keys.get("first").unwrap()[0],
        "e2e-first/test.sqlite",
        exported_str,
        &format!("Exported e2e-first/test.sqlite to {exported_str}"),
    )?;
    assert!(
        exported.metadata()?.len() > 0,
        "exported database should be non-empty"
    );

    // A user without read access cannot export.
    export_database(
        config_path,
        &api_keys.get("second").unwrap()[0],
        "e2e-first/test.sqlite",
        &work_dir.join("denied.sqlite").to_string_lossy(),
        "Error",
    )?;

    // Seed a new database from the exported file. The contents should
    // be queryable immediately. The exact row count depends on test
    // order (earlier tests insert rows into the source DB), so we
    // just verify that the seeded table has at least one row.
    create_database_from_file(
        config_path,
        &api_keys.get("first").unwrap()[0],
        "e2e-first/imported.sqlite",
        exported_str,
        "Successfully created e2e-first/imported.sqlite",
    )?;
    query(
        config_path,
        &api_keys.get("first").unwrap()[0],
        "SELECT count(*) > 0 AS has_rows FROM test_table;",
        "e2e-first/imported.sqlite",
        "csv",
        "has_rows\n1\n\nRows: 1",
    )?;

    // Seeding from a non-SQLite file is rejected and the database
    // record is not created (so the slug stays available for retry).
    let bad_file = work_dir.join("not-a-db.bin");
    fs::write(&bad_file, b"this is definitely not a sqlite database")?;
    create_database_from_file(
        config_path,
        &api_keys.get("first").unwrap()[0],
        "e2e-first/bad-seed.sqlite",
        bad_file.to_str().unwrap(),
        "Error",
    )?;

    Ok(())
}
