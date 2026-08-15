use crate::utils::ayb::{create_database_from_file, export_database, query};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

/// Round-trip both engines through the export and create-from-file
/// endpoints. This test depends on the databases populated by
/// `test_create_and_query_db` and `test_create_and_query_duckdb`, so it
/// must run after both.
pub fn test_export_and_import(
    test_type: &str,
    config_path: &str,
    api_keys: &HashMap<String, Vec<String>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let work_dir = PathBuf::from(format!("tests/ayb_data_{test_type}/export_import"));
    fs::create_dir_all(&work_dir)?;
    let first_key = &api_keys.get("first").unwrap()[0];

    let exported_sqlite = work_dir.join("exported.sqlite");
    let exported_sqlite_str = exported_sqlite.to_str().unwrap();
    let exported_duckdb = work_dir.join("exported.duckdb");
    let exported_duckdb_str = exported_duckdb.to_str().unwrap();

    // Owner can export their databases. Each exported file should be a
    // valid database of its engine with the rows previously inserted.
    export_database(
        config_path,
        first_key,
        "e2e-first/test.sqlite",
        exported_sqlite_str,
        &format!("Exported e2e-first/test.sqlite to {exported_sqlite_str}"),
    )?;
    assert!(
        exported_sqlite.metadata()?.len() > 0,
        "exported SQLite database should be non-empty"
    );

    export_database(
        config_path,
        first_key,
        "e2e-first/test.duckdb",
        exported_duckdb_str,
        &format!("Exported e2e-first/test.duckdb to {exported_duckdb_str}"),
    )?;
    assert!(
        exported_duckdb.metadata()?.len() > 0,
        "exported DuckDB database should be non-empty"
    );

    // A user without read access cannot export.
    export_database(
        config_path,
        &api_keys.get("second").unwrap()[0],
        "e2e-first/test.sqlite",
        &work_dir.join("denied.sqlite").to_string_lossy(),
        "Error",
    )?;

    // Seed new databases from the exported files. The contents should be
    // queryable immediately. The exact row count depends on test order
    // (earlier tests insert rows into the source databases), so we just
    // verify that the seeded table has at least one row.
    create_database_from_file(
        config_path,
        first_key,
        "e2e-first/imported.sqlite",
        "sqlite",
        exported_sqlite_str,
        "Successfully created e2e-first/imported.sqlite",
    )?;
    query(
        config_path,
        first_key,
        "SELECT count(*) > 0 AS has_rows FROM test_table;",
        "e2e-first/imported.sqlite",
        "csv",
        "has_rows\n1\n\nRows: 1",
    )?;

    create_database_from_file(
        config_path,
        first_key,
        "e2e-first/imported.duckdb",
        "duckdb",
        exported_duckdb_str,
        "Successfully created e2e-first/imported.duckdb",
    )?;
    query(
        config_path,
        first_key,
        "SELECT count(*) > 0 AS has_rows FROM test_table;",
        "e2e-first/imported.duckdb",
        "csv",
        "has_rows\ntrue\n\nRows: 1",
    )?;

    // Seeding from a file that isn't a database at all is rejected, and
    // the database record is not created (so the slug stays available
    // for a retry).
    let bad_file = work_dir.join("not-a-db.bin");
    fs::write(&bad_file, b"this is definitely not a database")?;
    create_database_from_file(
        config_path,
        first_key,
        "e2e-first/bad-seed.sqlite",
        "sqlite",
        bad_file.to_str().unwrap(),
        "Error",
    )?;

    // Seeding from a valid database of the *wrong* engine is also
    // rejected, in both directions.
    create_database_from_file(
        config_path,
        first_key,
        "e2e-first/wrong-engine.duckdb",
        "duckdb",
        exported_sqlite_str,
        "Error",
    )?;
    create_database_from_file(
        config_path,
        first_key,
        "e2e-first/wrong-engine.sqlite",
        "sqlite",
        exported_duckdb_str,
        "Error",
    )?;

    Ok(())
}
