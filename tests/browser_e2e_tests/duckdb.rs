use playwright_rs::{ClickOptions, FillOptions, GotoOptions, Page, WaitForOptions};
use std::error::Error;
use std::time::Duration;

/// Fill the web query editor with `sql` (clearing any previous query
/// first), click "Run query", and give htmx a moment to issue the
/// request. The query editor submits asynchronously (htmx swaps the
/// results panel), so we settle briefly to keep a following write from
/// racing the previous one.
async fn run_query(page: &Page, sql: &str) -> Result<(), Box<dyn Error>> {
    page.locator("textarea[name='query']")
        .await
        .first()
        .fill("", Some(FillOptions::builder().timeout(1000.0).build()))
        .await?;
    page.locator("textarea[name='query']")
        .await
        .first()
        .fill(sql, Some(FillOptions::builder().timeout(1000.0).build()))
        .await?;
    page.locator("button:has-text('Run query')")
        .await
        .first()
        .click(Some(ClickOptions::builder().timeout(5000.0).build()))
        .await?;
    tokio::time::sleep(Duration::from_millis(750)).await;
    Ok(())
}

/// Wait until the query-results panel contains `needle`. The results are
/// filled in asynchronously by htmx after "Run query", so we poll the
/// panel rather than reading it immediately (which would race the swap).
async fn wait_for_results_containing(page: &Page, needle: &str) -> Result<(), Box<dyn Error>> {
    page.locator(&format!("#query-results:has-text('{needle}')"))
        .await
        .first()
        .wait_for(Some(WaitForOptions::builder().timeout(10000.0).build()))
        .await?;
    Ok(())
}

/// A light DuckDB browser flow paralleling the SQLite UI tests: create a
/// DuckDB database through the web form (selecting the DuckDB engine),
/// insert and query rows, wait for the snapshot daemon to capture two
/// distinct states, then restore the earlier snapshot through the
/// Snapshots tab and confirm the later insert is gone. Intentionally
/// lighter than the SQLite browser tests -- assertion-based, no
/// screenshot comparisons -- since DuckDB's non-UI behavior is already
/// covered exhaustively by the API-level e2e tests.
pub async fn test_duckdb_flow(
    page: &Page,
    username: &str,
    base_url: &str,
) -> Result<(), Box<dyn Error>> {
    // Step 1: Go to the entity dashboard and open the create-database form.
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

    // Step 2: Name the database and select the DuckDB engine (the form
    // defaults to SQLite, so we click the DuckDB button explicitly).
    page.locator("input[name='database_slug']")
        .await
        .first()
        .fill(
            "test.duckdb",
            Some(FillOptions::builder().timeout(1000.0).build()),
        )
        .await?;

    page.locator("button[data-value='duckdb']")
        .await
        .first()
        .click(Some(ClickOptions::builder().timeout(3000.0).build()))
        .await?;

    // Step 3: Submit. On success the server responds with an HX-Redirect
    // that htmx follows asynchronously, so wait for the database page to
    // load (its query editor appears) before asserting the title.
    page.locator("button[type='submit']:has-text('Create database')")
        .await
        .first()
        .click(Some(ClickOptions::builder().timeout(5000.0).build()))
        .await?;

    page.locator("textarea[name='query']")
        .await
        .first()
        .wait_for(Some(WaitForOptions::builder().timeout(10000.0).build()))
        .await?;

    let database_page_title = format!("Explore {}/test.duckdb - ayb", username);
    assert_eq!(page.title().await?, database_page_title);

    // Step 4: Create a table and insert the same two rows as the SQLite
    // tests. (DuckDB reports an affected-row Count for INSERT; we don't
    // assert on it here.)
    run_query(
        page,
        "CREATE TABLE test_table(fname varchar, lname varchar);",
    )
    .await?;
    run_query(
        page,
        "INSERT INTO test_table (fname, lname) VALUES ('the first', 'the last');",
    )
    .await?;
    run_query(
        page,
        "INSERT INTO test_table (fname, lname) VALUES ('the first2', 'the last2');",
    )
    .await?;

    // Step 5: Query the two rows back and verify the data rendered.
    run_query(page, "SELECT * FROM test_table;").await?;
    wait_for_results_containing(page, "the first2").await?;
    let results = page.locator("#query-results").await.inner_text().await?;
    assert!(
        results.contains("the first") && results.contains("the last"),
        "DuckDB query results should contain the first inserted row, got: {results}"
    );
    assert!(
        results.contains("the first2") && results.contains("the last2"),
        "DuckDB query results should contain the second inserted row, got: {results}"
    );

    // Step 6: Wait for the snapshot daemon (runs every 2s on change) to
    // capture the two-row state.
    tokio::time::sleep(Duration::from_millis(4000)).await;

    // Step 7: Insert a third row and confirm it's present before we
    // snapshot the three-row state.
    run_query(
        page,
        "INSERT INTO test_table (fname, lname) VALUES ('snapshot', 'test');",
    )
    .await?;
    run_query(page, "SELECT * FROM test_table;").await?;
    wait_for_results_containing(page, "snapshot").await?;

    // Step 8: Wait for a second, distinct snapshot of the three-row state.
    tokio::time::sleep(Duration::from_millis(4000)).await;

    // Step 9: Open the Snapshots tab and restore the second-newest
    // snapshot (the two-row state; snapshots are sorted newest-first).
    page.locator("a[href='#snapshots']")
        .await
        .first()
        .click(Some(ClickOptions::builder().timeout(5000.0).build()))
        .await?;

    page.locator("tbody tr:nth-child(2) button[title='Restore from this snapshot']")
        .await
        .first()
        .click(Some(ClickOptions::builder().timeout(5000.0).build()))
        .await?;

    // Allow the confirmation modal to animate in, then confirm the restore.
    tokio::time::sleep(Duration::from_millis(200)).await;
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

    // Step 10: Reload the database page and verify the third ('snapshot')
    // row is gone while the original two rows remain.
    page.goto(
        &format!("{}/{}/test.duckdb", base_url, username),
        Some(GotoOptions::new().timeout(Duration::from_millis(5000))),
    )
    .await?;

    run_query(page, "SELECT * FROM test_table;").await?;
    wait_for_results_containing(page, "the first").await?;
    let final_results = page.locator("#query-results").await.inner_text().await?;
    assert!(
        final_results.contains("the first2"),
        "The original rows should remain after restore, got: {final_results}"
    );
    assert!(
        !final_results.contains("snapshot"),
        "The 'snapshot' row should be gone after restore, got: {final_results}"
    );

    Ok(())
}
