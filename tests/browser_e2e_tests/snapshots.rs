use crate::utils::browser::BrowserHelpers;
use playwright::api::Page;
use std::error::Error;

pub async fn test_snapshots_flow(
    page: &Page,
    username: &str,
    base_url: &str,
) -> Result<(), Box<dyn Error>> {
    // Step 1: Navigate to the existing database (test.sqlite created in create_and_query_database.rs)
    let database_page_title = format!("Explore {}/test.sqlite - ayb", username);
    let database_url = format!("{}/{}/test.sqlite", base_url, username);

    page.goto_builder(&database_url)
        .timeout(5000.0)
        .goto()
        .await?;

    // Wait for page to load
    tokio::time::sleep(std::time::Duration::from_millis(1000)).await;

    // Verify we're on the correct database page
    assert_eq!(page.title().await?, database_page_title);

    BrowserHelpers::screenshot_compare(&page, "snapshots_database_page_start", &[]).await?;

    // Step 2: Query to check existing row count (should be 2 from create_and_query_database.rs)
    let count_query = "SELECT COUNT(*) FROM test_table;";

    page.fill_builder("textarea[name='query']", count_query)
        .timeout(1000.0)
        .fill()
        .await?;

    BrowserHelpers::screenshot_compare(&page, "snapshots_count_query", &[]).await?;

    page.click_builder("button:has-text('Run query')")
        .timeout(5000.0)
        .click()
        .await?;

    tokio::time::sleep(std::time::Duration::from_millis(1000)).await;

    // Verify we have 2 rows initially
    let page_text = page.inner_text("body", None).await?;
    assert!(page_text.contains("2"), "Initial count should show 2 rows");

    BrowserHelpers::screenshot_compare(&page, "snapshots_initial_count", &[]).await?;

    // Step 3: Wait for automatic snapshot to be created (snapshots are auto-created after DB changes)
    // The system takes snapshots automatically every 2 seconds when database changes
    tokio::time::sleep(std::time::Duration::from_millis(3000)).await;

    // Step 4: Insert a new row
    let insert_query = "INSERT INTO test_table (fname, lname) VALUES (\"snapshot\", \"test\");";

    page.fill_builder("textarea[name='query']", "")
        .timeout(1000.0)
        .fill()
        .await?;

    page.fill_builder("textarea[name='query']", insert_query)
        .timeout(1000.0)
        .fill()
        .await?;

    BrowserHelpers::screenshot_compare(&page, "snapshots_insert_query", &[]).await?;

    page.click_builder("button:has-text('Run query')")
        .timeout(5000.0)
        .click()
        .await?;

    tokio::time::sleep(std::time::Duration::from_millis(1000)).await;

    BrowserHelpers::screenshot_compare(&page, "snapshots_row_inserted", &[]).await?;

    // Step 5: Verify we now have 3 rows
    page.fill_builder("textarea[name='query']", count_query)
        .timeout(1000.0)
        .fill()
        .await?;

    page.click_builder("button:has-text('Run query')")
        .timeout(5000.0)
        .click()
        .await?;

    // Screenshot immediately after query execution (following create_and_query_database pattern)
    BrowserHelpers::screenshot_compare(&page, "snapshots_count_after_insert", &[]).await?;

    // Now read the results from the specific query results element
    let query_results = page.inner_text("#query-results", None).await?;
    assert!(
        query_results.contains("3"),
        "Count after insert should show 3 rows"
    );

    // Step 6: Sleep to allow automatic snapshot after insert (as requested by user)
    tokio::time::sleep(std::time::Duration::from_millis(3000)).await;

    // Step 7: Click the Snapshots tab to see available snapshots
    // This triggers the proper tab switching and AJAX loading of snapshots
    page.click_builder("a[href='#snapshots']")
        .timeout(5000.0)
        .click()
        .await?;

    // Wait for the AJAX loadSnapshots() function to complete
    tokio::time::sleep(std::time::Duration::from_millis(3000)).await;

    BrowserHelpers::screenshot_compare(&page, "snapshots_list_page", &[]).await?;

    // Step 8: Click the restore button for the SECOND snapshot (older one with 2 rows)
    // Snapshots are sorted newest-first, so we need the second table row's button
    page.click_builder("tbody tr:nth-child(2) button[title='Restore from this snapshot']")
        .timeout(5000.0)
        .click()
        .await?;

    // Wait for confirmation modal animation to complete
    tokio::time::sleep(std::time::Duration::from_millis(3000)).await;

    BrowserHelpers::screenshot_compare(&page, "snapshots_confirmation_modal", &[]).await?;

    // Step 9: Wait for the actual restore button to be clickable and click it
    page.wait_for_selector_builder("#confirm-restore-btn")
        .timeout(15000.0)
        .wait_for_selector()
        .await?;
    
    page.click_builder("#confirm-restore-btn")
        .timeout(15000.0)
        .click()
        .await?;

    tokio::time::sleep(std::time::Duration::from_millis(3000)).await;

    BrowserHelpers::screenshot_compare(&page, "snapshots_restored", &[]).await?;

    // Step 10: Navigate back to database page to verify restoration
    let database_url = format!("{}/{}/test.sqlite", base_url, username);
    page.goto_builder(&database_url)
        .timeout(5000.0)
        .goto()
        .await?;

    tokio::time::sleep(std::time::Duration::from_millis(1000)).await;

    // Step 11: Verify we're back to 2 rows (one less than the 3 we had)
    page.fill_builder("textarea[name='query']", count_query)
        .timeout(1000.0)
        .fill()
        .await?;

    page.click_builder("button:has-text('Run query')")
        .timeout(5000.0)
        .click()
        .await?;

    tokio::time::sleep(std::time::Duration::from_millis(1000)).await;

    let page_text_after_restore = page.inner_text("body", None).await?;
    assert!(
        page_text_after_restore.contains("2"),
        "Count after snapshot restore should show 2 rows (one less than before)"
    );

    BrowserHelpers::screenshot_compare(&page, "snapshots_final_count", &[]).await?;

    // Step 12: Verify the inserted row is gone
    let select_query = "SELECT * FROM test_table;";

    page.fill_builder("textarea[name='query']", select_query)
        .timeout(1000.0)
        .fill()
        .await?;

    page.click_builder("button:has-text('Run query')")
        .timeout(5000.0)
        .click()
        .await?;

    tokio::time::sleep(std::time::Duration::from_millis(1000)).await;

    let final_page_text = page.inner_text("body", None).await?;
    assert!(
        !final_page_text.contains("snapshot"),
        "The inserted row with 'snapshot' should be gone after restore"
    );

    BrowserHelpers::screenshot_compare(&page, "snapshots_test_complete", &[]).await?;

    Ok(())
}
