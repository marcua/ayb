use crate::utils::browser::BrowserHelpers;
use playwright_rs::{ClickOptions, FillOptions, GotoOptions, Page, WaitForOptions};
use std::error::Error;
use std::time::Duration;

/// A DuckDB browser flow paralleling the SQLite UI tests: create a DuckDB
/// database through the web form (selecting the DuckDB engine), insert
/// and query rows, wait for the snapshot daemon to capture two distinct
/// states, then restore the earlier snapshot through the Snapshots tab
/// and confirm the later insert is gone. Mirrors the constructs used in
/// create_and_query_database.rs and snapshots.rs.
pub async fn test_duckdb_flow(
    page: &Page,
    username: &str,
    base_url: &str,
) -> Result<(), Box<dyn Error>> {
    // Step 1: Navigate to the entity dashboard and open the create
    // database form.
    page.goto(
        &format!("{}/{}", base_url, username),
        Some(GotoOptions::new().timeout(Duration::from_millis(5000))),
    )
    .await?;

    page.locator("button:has-text('Create database')")
        .await
        .first()
        .click(Some(ClickOptions::builder().timeout(3000.0).build()))
        .await?;

    // Screenshot of the create database form
    BrowserHelpers::screenshot_compare(page, "duckdb_create_database_form", &[]).await?;

    // Step 2: Fill in database name
    page.locator("input[name='database_slug']")
        .await
        .first()
        .fill(
            "test.duckdb",
            Some(FillOptions::builder().timeout(1000.0).build()),
        )
        .await?;

    // Step 3: Select the DuckDB engine (the form defaults to SQLite)
    page.locator("button[data-value='duckdb']")
        .await
        .first()
        .click(Some(ClickOptions::builder().timeout(3000.0).build()))
        .await?;

    // Screenshot of filled form
    BrowserHelpers::screenshot_compare(page, "duckdb_database_form_filled", &[]).await?;

    // Step 4: Submit the create database form
    page.locator("button[type='submit']:has-text('Create database')")
        .await
        .first()
        .click(Some(ClickOptions::builder().timeout(5000.0).build()))
        .await?;

    // Screenshot after database creation
    BrowserHelpers::screenshot_compare(page, "duckdb_database_created", &[]).await?;

    // Step 5: Ensure we're on the database page
    let database_page_title = format!("Explore {}/test.duckdb - ayb", username);

    // Verify we're on the database page
    assert_eq!(page.title().await?, database_page_title);

    // Step 6: Create the same table as in the SQLite tests
    let create_table_query = "CREATE TABLE test_table(fname varchar, lname varchar);";

    page.locator("textarea[name='query']")
        .await
        .first()
        .fill(
            create_table_query,
            Some(FillOptions::builder().timeout(1000.0).build()),
        )
        .await?;

    // Screenshot with create table query
    BrowserHelpers::screenshot_compare(page, "duckdb_create_table_query", &[]).await?;

    // Run the create table query
    page.locator("button:has-text('Run query')")
        .await
        .first()
        .click(Some(ClickOptions::builder().timeout(5000.0).build()))
        .await?;

    // Screenshot after table creation
    BrowserHelpers::screenshot_compare(page, "duckdb_table_created", &[]).await?;

    // Step 7: Insert data
    let insert_query1 = "INSERT INTO test_table (fname, lname) VALUES ('the first', 'the last');";

    // Clear previous query and enter insert query
    page.locator("textarea[name='query']")
        .await
        .first()
        .fill("", Some(FillOptions::builder().timeout(1000.0).build()))
        .await?;

    page.locator("textarea[name='query']")
        .await
        .first()
        .fill(
            insert_query1,
            Some(FillOptions::builder().timeout(1000.0).build()),
        )
        .await?;

    page.locator("button:has-text('Run query')")
        .await
        .first()
        .click(Some(ClickOptions::builder().timeout(5000.0).build()))
        .await?;

    // Let the first insert's request finish before submitting the next
    // one. The query form posts via htmx with no hx-sync, so two submits
    // fired back-to-back from the same form overlap and one of the
    // inserts is lost. Every other step here is separated by a
    // screenshot_compare (which settles briefly); these two are not.
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Insert second row
    let insert_query2 = "INSERT INTO test_table (fname, lname) VALUES ('the first2', 'the last2');";

    page.locator("textarea[name='query']")
        .await
        .first()
        .fill("", Some(FillOptions::builder().timeout(1000.0).build()))
        .await?;

    page.locator("textarea[name='query']")
        .await
        .first()
        .fill(
            insert_query2,
            Some(FillOptions::builder().timeout(1000.0).build()),
        )
        .await?;

    page.locator("button:has-text('Run query')")
        .await
        .first()
        .click(Some(ClickOptions::builder().timeout(5000.0).build()))
        .await?;

    // Screenshot after data insertion
    BrowserHelpers::screenshot_compare(page, "duckdb_data_inserted", &[]).await?;

    // Step 8: Query the data
    let select_query = "SELECT * FROM test_table;";

    page.locator("textarea[name='query']")
        .await
        .first()
        .fill("", Some(FillOptions::builder().timeout(1000.0).build()))
        .await?;

    page.locator("textarea[name='query']")
        .await
        .first()
        .fill(
            select_query,
            Some(FillOptions::builder().timeout(1000.0).build()),
        )
        .await?;

    // Screenshot with select query
    BrowserHelpers::screenshot_compare(page, "duckdb_select_query", &[]).await?;

    page.locator("button:has-text('Run query')")
        .await
        .first()
        .click(Some(ClickOptions::builder().timeout(5000.0).build()))
        .await?;

    // Screenshot of query results table
    BrowserHelpers::screenshot_compare(page, "duckdb_query_results", &[]).await?;

    // Step 9: Verify the results contain the expected data with exact
    // string matching (rendering is engine-agnostic for the same data)
    let query_results = page.locator("#query-results").await.inner_text().await?;

    let expected_results = "Download CSV\nDownload JSON\nfname\tlname\nthe first\tthe last\nthe first2\tthe last2\n2 rows";

    assert_eq!(
        query_results.trim(),
        expected_results,
        "Query results should exactly match expected content"
    );

    // Step 10: Wait for automatic snapshot to be created (snapshots are
    // auto-created after DB changes, every 2 seconds)
    tokio::time::sleep(Duration::from_millis(3000)).await;

    // Step 11: Insert a new row
    let insert_query = "INSERT INTO test_table (fname, lname) VALUES ('snapshot', 'test');";

    page.locator("textarea[name='query']")
        .await
        .first()
        .fill("", Some(FillOptions::builder().timeout(1000.0).build()))
        .await?;

    page.locator("textarea[name='query']")
        .await
        .first()
        .fill(
            insert_query,
            Some(FillOptions::builder().timeout(1000.0).build()),
        )
        .await?;

    BrowserHelpers::screenshot_compare(page, "duckdb_snapshots_insert_query", &[]).await?;

    page.locator("button:has-text('Run query')")
        .await
        .first()
        .click(Some(ClickOptions::builder().timeout(5000.0).build()))
        .await?;

    BrowserHelpers::screenshot_compare(page, "duckdb_snapshots_row_inserted", &[]).await?;

    // Step 12: Verify we now have 3 rows
    let count_query = "SELECT COUNT(*) FROM test_table;";

    page.locator("textarea[name='query']")
        .await
        .first()
        .fill(
            count_query,
            Some(FillOptions::builder().timeout(1000.0).build()),
        )
        .await?;

    page.locator("button:has-text('Run query')")
        .await
        .first()
        .click(Some(ClickOptions::builder().timeout(5000.0).build()))
        .await?;

    BrowserHelpers::screenshot_compare(page, "duckdb_snapshots_count_after_insert", &[]).await?;

    let query_results = page.locator("#query-results").await.inner_text().await?;
    assert!(
        query_results.contains("3"),
        "Count after insert should show 3 rows"
    );

    // Step 13: Sleep to allow automatic snapshot after insert
    tokio::time::sleep(Duration::from_millis(3000)).await;

    // Step 14: Click the Snapshots tab to see available snapshots
    page.locator("a[href='#snapshots']")
        .await
        .first()
        .click(Some(ClickOptions::builder().timeout(5000.0).build()))
        .await?;

    BrowserHelpers::screenshot_compare(page, "duckdb_snapshots_list_page", &[]).await?;

    // Step 15: Click the restore button for the SECOND snapshot (older one
    // with 2 rows). Snapshots are sorted newest-first, so we need the
    // second table row's button.
    page.locator("tbody tr:nth-child(2) button[title='Restore from this snapshot']")
        .await
        .first()
        .click(Some(ClickOptions::builder().timeout(5000.0).build()))
        .await?;

    // Extra delay for modal animation to complete
    tokio::time::sleep(Duration::from_millis(200)).await;

    BrowserHelpers::screenshot_compare(page, "duckdb_snapshots_confirmation_modal", &[]).await?;

    // Step 16: Wait for the actual restore button to be clickable and
    // click it
    page.locator("#confirm-restore-btn")
        .await
        .first()
        .wait_for(Some(WaitForOptions::builder().timeout(15000.0).build()))
        .await?;

    page.locator("#confirm-restore-btn")
        .await
        .first()
        .click(Some(ClickOptions::builder().timeout(15000.0).build()))
        .await?;

    BrowserHelpers::screenshot_compare(page, "duckdb_snapshots_restored", &[]).await?;

    // Step 17: Navigate back to database page to verify restoration
    page.goto(
        &format!("{}/{}/test.duckdb", base_url, username),
        Some(GotoOptions::new().timeout(Duration::from_millis(5000))),
    )
    .await?;

    // Step 18: Verify we're back to 2 rows (one less than the 3 we had)
    page.locator("textarea[name='query']")
        .await
        .first()
        .fill(
            count_query,
            Some(FillOptions::builder().timeout(1000.0).build()),
        )
        .await?;

    page.locator("button:has-text('Run query')")
        .await
        .first()
        .click(Some(ClickOptions::builder().timeout(5000.0).build()))
        .await?;

    BrowserHelpers::screenshot_compare(page, "duckdb_snapshots_final_count", &[]).await?;

    let page_text_after_restore = page.locator("#query-results").await.inner_text().await?;
    assert!(
        page_text_after_restore.contains("2"),
        "Count after snapshot restore should show 2 rows (one less than before)"
    );

    // Step 19: Verify the inserted row is gone
    page.locator("textarea[name='query']")
        .await
        .first()
        .fill(
            select_query,
            Some(FillOptions::builder().timeout(1000.0).build()),
        )
        .await?;

    page.locator("button:has-text('Run query')")
        .await
        .first()
        .click(Some(ClickOptions::builder().timeout(5000.0).build()))
        .await?;

    BrowserHelpers::screenshot_compare(page, "duckdb_snapshots_test_complete", &[]).await?;

    let final_page_text = page.locator("#query-results").await.inner_text().await?;
    assert!(
        !final_page_text.contains("snapshot"),
        "The inserted row with 'snapshot' should be gone after restore"
    );

    Ok(())
}
