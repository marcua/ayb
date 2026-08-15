use crate::utils::ayb::{create_database, export_database, import_database, query};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

/// Exercises export and import for both engines.
///
/// The rejection cases matter as much as the happy path: an import
/// replaces a live database, so every rejected import is also checked to
/// have left the target's contents intact.
///
/// Depends on the databases populated by `test_create_and_query_db` and
/// `test_create_and_query_duckdb`, so it must run after both.
pub fn test_export_and_import(
    test_type: &str,
    config_path: &str,
    api_keys: &HashMap<String, Vec<String>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let work_dir = PathBuf::from(format!("tests/ayb_data_{test_type}/export_import"));
    fs::create_dir_all(&work_dir)?;
    let first_key = &api_keys.get("first").unwrap()[0];
    let second_key = &api_keys.get("second").unwrap()[0];

    let exported_sqlite = work_dir.join("exported.sqlite");
    let exported_sqlite_str = exported_sqlite.to_str().unwrap();
    let exported_duckdb = work_dir.join("exported.duckdb");
    let exported_duckdb_str = exported_duckdb.to_str().unwrap();

    // The owner can export both databases, and each export is a
    // non-empty file.
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

    // A user without read access can't export.
    export_database(
        config_path,
        second_key,
        "e2e-first/test.sqlite",
        &work_dir.join("denied.sqlite").to_string_lossy(),
        "Error",
    )?;

    // Import each export into a fresh, empty database of the same
    // engine. The contents should be queryable immediately. The exact
    // row count depends on test order (earlier tests insert rows into
    // the source databases), so assert only that rows arrived.
    create_database(
        config_path,
        first_key,
        "e2e-first/imported.sqlite",
        "sqlite",
        "Successfully created e2e-first/imported.sqlite",
    )?;
    import_database(
        config_path,
        first_key,
        "e2e-first/imported.sqlite",
        exported_sqlite_str,
        "Imported",
    )?;
    query(
        config_path,
        first_key,
        "SELECT count(*) > 0 AS has_rows FROM test_table;",
        "e2e-first/imported.sqlite",
        "csv",
        "has_rows\n1\n\nRows: 1",
    )?;

    create_database(
        config_path,
        first_key,
        "e2e-first/imported.duckdb",
        "duckdb",
        "Successfully created e2e-first/imported.duckdb",
    )?;
    import_database(
        config_path,
        first_key,
        "e2e-first/imported.duckdb",
        exported_duckdb_str,
        "Imported",
    )?;
    query(
        config_path,
        first_key,
        "SELECT count(*) > 0 AS has_rows FROM test_table;",
        "e2e-first/imported.duckdb",
        "csv",
        "has_rows\ntrue\n\nRows: 1",
    )?;

    // A file that isn't a database at all is rejected, and the target
    // keeps the contents it had.
    let bad_file = work_dir.join("not-a-db.bin");
    fs::write(&bad_file, b"this is definitely not a database")?;
    import_database(
        config_path,
        first_key,
        "e2e-first/imported.sqlite",
        bad_file.to_str().unwrap(),
        "Error",
    )?;
    query(
        config_path,
        first_key,
        "SELECT count(*) > 0 AS has_rows FROM test_table;",
        "e2e-first/imported.sqlite",
        "csv",
        "has_rows\n1\n\nRows: 1",
    )?;

    // A truncated database file is rejected too: the header alone looks
    // plausible, so this only fails if the file is really opened and
    // checked rather than sniffed.
    let truncated = work_dir.join("truncated.sqlite");
    let full = fs::read(&exported_sqlite)?;
    fs::write(&truncated, &full[..full.len() / 2])?;
    import_database(
        config_path,
        first_key,
        "e2e-first/imported.sqlite",
        truncated.to_str().unwrap(),
        "Error",
    )?;
    query(
        config_path,
        first_key,
        "SELECT count(*) > 0 AS has_rows FROM test_table;",
        "e2e-first/imported.sqlite",
        "csv",
        "has_rows\n1\n\nRows: 1",
    )?;

    // A valid database of the *wrong* engine is rejected in both
    // directions, and neither target is disturbed.
    import_database(
        config_path,
        first_key,
        "e2e-first/imported.sqlite",
        exported_duckdb_str,
        "Error",
    )?;
    query(
        config_path,
        first_key,
        "SELECT count(*) > 0 AS has_rows FROM test_table;",
        "e2e-first/imported.sqlite",
        "csv",
        "has_rows\n1\n\nRows: 1",
    )?;

    import_database(
        config_path,
        first_key,
        "e2e-first/imported.duckdb",
        exported_sqlite_str,
        "Error",
    )?;
    query(
        config_path,
        first_key,
        "SELECT count(*) > 0 AS has_rows FROM test_table;",
        "e2e-first/imported.duckdb",
        "csv",
        "has_rows\ntrue\n\nRows: 1",
    )?;

    // Importing requires manage permission, not merely query access.
    import_database(
        config_path,
        second_key,
        "e2e-first/imported.sqlite",
        exported_sqlite_str,
        "Error",
    )?;

    Ok(())
}
