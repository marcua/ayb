use crate::utils::browser::BrowserHelpers;
use playwright_rs::{ClickOptions, FillOptions, GotoOptions, Page, WaitForOptions};
use std::error::Error;
use std::time::Duration;

/// Let a query submission's request finish before the next one is issued.
///
/// The query form posts via htmx from a single element with no `hx-sync`,
/// so a submit issued while another is in flight is queued rather than
/// sent, and the queued closure re-reads the textarea when it eventually
/// runs. A further overlapping submit dumps that queue outright (htmx's
/// default queue strategy is `last`). Either way a submission silently
/// produces no request of its own, which shows up downstream as a row
/// count that is short by one.
///
/// The `screenshot_compare` calls between steps settle briefly, but that
/// margin is incidental rather than designed, and it is too thin on a
/// loaded runner. Settling explicitly after every query submission
/// removes the whole class. A sleep rather than an extra
/// `screenshot_compare` is deliberate: the screenshot counter is global
/// and sequential, so inserting a screenshot mid-suite renumbers every
/// later reference PNG.
async fn settle_after_query() {
    tokio::time::sleep(Duration::from_millis(500)).await;
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
    settle_after_query().await;

    // Verify we have 2 rows initially
    let page_text = page.locator("#query-results").await.inner_text().await?;
    assert!(page_text.contains("2"), "Initial count should show 2 rows");

    // Step 3: Wait for automatic snapshot to be created (snapshots are auto-created after DB changes)
    // The system takes snapshots automatically every 2 seconds when database changes
    tokio::time::sleep(std::time::Duration::from_millis(3000)).await;

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
    settle_after_query().await;

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
    settle_after_query().await;

    // Now read the results from the specific query results element
    let query_results = page.locator("#query-results").await.inner_text().await?;
    assert!(
        query_results.contains("3"),
        "Count after insert should show 3 rows"
    );

    // Step 6: Sleep to allow automatic snapshot after insert
    tokio::time::sleep(std::time::Duration::from_millis(3000)).await;

    // Step 7: Click the Snapshots tab to see available snapshots
    // This triggers the proper tab switching and AJAX loading of snapshots
    page.locator("a[href='#snapshots']")
        .await
        .first()
        .click(Some(ClickOptions::builder().timeout(5000.0).build()))
        .await?;

    BrowserHelpers::screenshot_compare(page, "snapshots_list_page", &[]).await?;

    // Step 8: Click the restore button for the SECOND snapshot (older one with 2 rows)
    // Snapshots are sorted newest-first, so we need the second table row's button
    page.locator("tbody tr:nth-child(2) button[title='Restore from this snapshot']")
        .await
        .first()
        .click(Some(ClickOptions::builder().timeout(5000.0).build()))
        .await?;

    // Extra delay for modal animation to complete
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;

    BrowserHelpers::screenshot_compare(page, "snapshots_confirmation_modal", &[]).await?;

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
    settle_after_query().await;

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
    settle_after_query().await;

    let final_page_text = page.locator("#query-results").await.inner_text().await?;
    assert!(
        !final_page_text.contains("snapshot"),
        "The inserted row with 'snapshot' should be gone after restore"
    );

    Ok(())
}
