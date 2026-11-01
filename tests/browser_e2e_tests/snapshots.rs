use crate::utils::browser::BrowserHelpers;
use playwright_rs::{ClickOptions, FillOptions, GotoOptions, Page, WaitForOptions};
use std::error::Error;
use std::time::{Duration, Instant};

const SNAPSHOT_TIMEOUT: Duration = Duration::from_secs(30);
const SNAPSHOT_POLL_INTERVAL: Duration = Duration::from_millis(500);
// Automatic snapshots run every 2s (see the test server config).
const SNAPSHOTS_SETTLED_AFTER: Duration = Duration::from_secs(6);

/// Snapshot list cells whose contents change from run to run, hidden in
/// screenshots.
pub const SNAPSHOT_DATE_SELECTORS: &[&str] = &[
    "#snapshots-list-container tbody td:nth-child(2)",
    "#restore-snapshot-date",
];

/// Returns the IDs of a database's snapshots, newest first, as listed on
/// its Snapshots tab. `database_path` is `entity/database`.
async fn list_snapshot_ids(
    page: &Page,
    database_path: &str,
) -> Result<Vec<String>, Box<dyn Error>> {
    let script = format!(
        r#"(async () => {{
            const response = await fetch('/{database_path}/snapshots');
            const html = new DOMParser().parseFromString(await response.text(), 'text/html');
            return [...html.querySelectorAll('tbody tr code')].map(code => code.textContent.trim());
        }})()"#
    );
    let ids = page
        .evaluate::<serde_json::Value, serde_json::Value>(&script, None)
        .await?;
    Ok(serde_json::from_value(ids)?)
}

/// Waits until a database's current contents have been snapshotted and
/// returns its snapshot IDs, newest first. Automatic snapshots skip a
/// round while the previous one is still running, so under load they can
/// lag well behind a fixed sleep. A snapshot is only uploaded for new
/// contents, so once the current contents are captured the list stops
/// changing: wait for it to be non-empty and unchanged for a few rounds.
pub async fn wait_for_settled_snapshots(
    page: &Page,
    database_path: &str,
) -> Result<Vec<String>, Box<dyn Error>> {
    let deadline = Instant::now() + SNAPSHOT_TIMEOUT;
    let mut ids = list_snapshot_ids(page, database_path).await?;
    let mut unchanged_since = Instant::now();
    loop {
        if !ids.is_empty() && unchanged_since.elapsed() >= SNAPSHOTS_SETTLED_AFTER {
            return Ok(ids);
        }
        if Instant::now() > deadline {
            return Err(format!("Snapshots of {database_path} didn't settle: {ids:?}").into());
        }
        tokio::time::sleep(SNAPSHOT_POLL_INTERVAL).await;
        let latest = list_snapshot_ids(page, database_path).await?;
        if latest != ids {
            ids = latest;
            unchanged_since = Instant::now();
        }
    }
}

/// Waits until a snapshot that isn't in `previous` is the newest one, and
/// returns the database's snapshot IDs, newest first.
pub async fn wait_for_new_snapshot(
    page: &Page,
    database_path: &str,
    previous: &[String],
) -> Result<Vec<String>, Box<dyn Error>> {
    let deadline = Instant::now() + SNAPSHOT_TIMEOUT;
    loop {
        let ids = list_snapshot_ids(page, database_path).await?;
        if ids.first().is_some_and(|newest| !previous.contains(newest)) {
            return Ok(ids);
        }
        if Instant::now() > deadline {
            return Err(format!("No new snapshot of {database_path}: {ids:?}").into());
        }
        tokio::time::sleep(SNAPSHOT_POLL_INTERVAL).await;
    }
}

pub async fn test_snapshots_flow(
    page: &Page,
    username: &str,
    base_url: &str,
) -> Result<(), Box<dyn Error>> {
    // Step 1: Navigate to the existing database (test.sqlite created in create_and_query_database.rs)
    let database_page_title = format!("Explore {}/test.sqlite - ayb", username);
    let database_url = format!("{}/{}/test.sqlite", base_url, username);

    page.goto(
        &database_url,
        Some(GotoOptions::new().timeout(Duration::from_millis(5000))),
    )
    .await?;

    // Verify we're on the correct database page
    assert_eq!(page.title().await?, database_page_title);

    BrowserHelpers::screenshot_compare(page, "snapshots_database_page_start", &[]).await?;

    // Step 2: Query to check existing row count (should be 2 from create_and_query_database.rs)
    let count_query = "SELECT COUNT(*) FROM test_table;";

    page.locator("textarea[name='query']")
        .await
        .first()
        .fill(
            count_query,
            Some(FillOptions::builder().timeout(1000.0).build()),
        )
        .await?;

    BrowserHelpers::screenshot_compare(page, "snapshots_count_query", &[]).await?;

    page.locator("button:has-text('Run query')")
        .await
        .first()
        .click(Some(ClickOptions::builder().timeout(5000.0).build()))
        .await?;

    BrowserHelpers::screenshot_compare(page, "snapshots_initial_count", &[]).await?;

    // Verify we have 2 rows initially
    let page_text = page.locator("#query-results").await.inner_text().await?;
    assert!(page_text.contains("2"), "Initial count should show 2 rows");

    // Step 3: Wait for an automatic snapshot of the current (2-row) contents
    let database_path = format!("{}/test.sqlite", username);
    let snapshots_before_insert = wait_for_settled_snapshots(page, &database_path).await?;

    // Step 4: Insert a new row
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

    BrowserHelpers::screenshot_compare(page, "snapshots_insert_query", &[]).await?;

    page.locator("button:has-text('Run query')")
        .await
        .first()
        .click(Some(ClickOptions::builder().timeout(5000.0).build()))
        .await?;

    BrowserHelpers::screenshot_compare(page, "snapshots_row_inserted", &[]).await?;

    // Step 5: Verify we now have 3 rows
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

    // Screenshot immediately after query execution (following create_and_query_database pattern)
    BrowserHelpers::screenshot_compare(page, "snapshots_count_after_insert", &[]).await?;

    // Now read the results from the specific query results element
    let query_results = page.locator("#query-results").await.inner_text().await?;
    assert!(
        query_results.contains("3"),
        "Count after insert should show 3 rows"
    );

    // Step 6: Wait for an automatic snapshot of the 3-row contents. The
    // restore below picks the second-newest snapshot, which must then be
    // the 2-row one.
    let snapshots_after_insert =
        wait_for_new_snapshot(page, &database_path, &snapshots_before_insert).await?;
    assert_eq!(
        snapshots_after_insert.get(1),
        snapshots_before_insert.first(),
        "The pre-insert snapshot should be second-newest"
    );

    // Step 7: Click the Snapshots tab to see available snapshots
    // This triggers the proper tab switching and AJAX loading of snapshots
    page.locator("a[href='#snapshots']")
        .await
        .first()
        .click(Some(ClickOptions::builder().timeout(5000.0).build()))
        .await?;

    // Wait for the list to render so its dates exist to be hidden
    page.locator("#snapshots-list-container tbody tr:nth-child(2)")
        .await
        .first()
        .wait_for(Some(WaitForOptions::builder().timeout(5000.0).build()))
        .await?;

    BrowserHelpers::screenshot_compare(page, "snapshots_list_page", SNAPSHOT_DATE_SELECTORS)
        .await?;

    // Step 8: Click the restore button for the SECOND snapshot (older one with 2 rows)
    // Snapshots are sorted newest-first, so we need the second table row's button
    page.locator("tbody tr:nth-child(2) button[title='Restore from this snapshot']")
        .await
        .first()
        .click(Some(ClickOptions::builder().timeout(5000.0).build()))
        .await?;

    // Extra delay for modal animation to complete
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;

    BrowserHelpers::screenshot_compare(
        page,
        "snapshots_confirmation_modal",
        SNAPSHOT_DATE_SELECTORS,
    )
    .await?;

    // Step 9: Wait for the actual restore button to be clickable and click it
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

    BrowserHelpers::screenshot_compare(page, "snapshots_restored", &[]).await?;

    // Step 10: Navigate back to database page to verify restoration
    let database_url = format!("{}/{}/test.sqlite", base_url, username);
    page.goto(
        &database_url,
        Some(GotoOptions::new().timeout(Duration::from_millis(5000))),
    )
    .await?;

    // Step 11: Verify we're back to 2 rows (one less than the 3 we had)
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

    BrowserHelpers::screenshot_compare(page, "snapshots_final_count", &[]).await?;

    let page_text_after_restore = page.locator("#query-results").await.inner_text().await?;
    assert!(
        page_text_after_restore.contains("2"),
        "Count after snapshot restore should show 2 rows (one less than before)"
    );

    // Step 12: Verify the inserted row is gone
    let select_query = "SELECT * FROM test_table;";

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

    BrowserHelpers::screenshot_compare(page, "snapshots_test_complete", &[]).await?;

    let final_page_text = page.locator("#query-results").await.inner_text().await?;
    assert!(
        !final_page_text.contains("snapshot"),
        "The inserted row with 'snapshot' should be gone after restore"
    );

    Ok(())
}
