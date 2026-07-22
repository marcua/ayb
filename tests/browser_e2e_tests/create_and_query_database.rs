use crate::utils::browser::BrowserHelpers;
use playwright_rs::{ClickOptions, FillOptions, Page};
use std::error::Error;
use std::fs;
use std::path::Path;

pub async fn test_create_and_query_database_flow(
    page: &Page,
    username: &str,
) -> Result<(), Box<dyn Error>> {
    // Step 1: Verify we're on the user dashboard and take initial screenshot
    // After profile test, the display name has changed from username to "Entity 0"
    let expected_title = "Entity 0 - ayb";
    assert_eq!(page.title().await?, expected_title);

    BrowserHelpers::screenshot_compare(page, "dashboard_before_database_creation", &[]).await?;

    // Step 2: Click on "Create database" button
    page.locator("button:has-text('Create database')")
        .await
        .first()
        .click(Some(ClickOptions::builder().timeout(3000.0).build()))
        .await?;

    // Screenshot of the create database form
    BrowserHelpers::screenshot_compare(page, "create_database_form", &[]).await?;

    // Step 3: Fill in database name (using similar name to e2e tests)
    page.locator("input[name='database_slug']")
        .await
        .first()
        .fill(
            "test.sqlite",
            Some(FillOptions::builder().timeout(1000.0).build()),
        )
        .await?;

    // Screenshot of filled form
    BrowserHelpers::screenshot_compare(page, "database_form_filled", &[]).await?;

    // Step 4: Submit the create database form
    page.locator("button[type='submit']:has-text('Create database')")
        .await
        .first()
        .click(Some(ClickOptions::builder().timeout(5000.0).build()))
        .await?;

    // Screenshot after database creation
    BrowserHelpers::screenshot_compare(page, "database_created", &[]).await?;

    // Step 5: Ensure we're on the database page
    let database_page_title = format!("Explore {}/test.sqlite - ayb", username);

    // Verify we're on the database page
    assert_eq!(page.title().await?, database_page_title);

    // Step 6: Create the same table as in e2e tests
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
    BrowserHelpers::screenshot_compare(page, "create_table_query", &[]).await?;

    // Run the create table query
    page.locator("button:has-text('Run query')")
        .await
        .first()
        .click(Some(ClickOptions::builder().timeout(5000.0).build()))
        .await?;

    // Screenshot after table creation
    BrowserHelpers::screenshot_compare(page, "table_created", &[]).await?;

    // Step 7: Insert data
    let insert_query1 =
        "INSERT INTO test_table (fname, lname) VALUES (\"the first\", \"the last\");";

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

    // Insert second row
    let insert_query2 =
        "INSERT INTO test_table (fname, lname) VALUES (\"the first2\", \"the last2\");";

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
    BrowserHelpers::screenshot_compare(page, "data_inserted", &[]).await?;

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
    BrowserHelpers::screenshot_compare(page, "select_query", &[]).await?;

    page.locator("button:has-text('Run query')")
        .await
        .first()
        .click(Some(ClickOptions::builder().timeout(5000.0).build()))
        .await?;

    // Screenshot of query results table
    BrowserHelpers::screenshot_compare(page, "query_results", &[]).await?;

    // Step 9: Verify the results contain the expected data with exact string matching
    let query_results = page.locator("#query-results").await.inner_text().await?;

    // Define the expected complete query results content
    let expected_results = "Download CSV\nDownload JSON\nfname\tlname\nthe first\tthe last\nthe first2\tthe last2\n2 rows";

    // Assert exact match of the query results content
    assert_eq!(
        query_results.trim(),
        expected_results,
        "Query results should exactly match expected content"
    );

    // Step 10: Test CSV download functionality
    let download_waiter = page.expect_download(Some(3000.0)).await?;
    page.locator("button:has-text('Download CSV')")
        .await
        .first()
        .click(Some(ClickOptions::builder().timeout(3000.0).build()))
        .await?;
    let download = download_waiter.wait().await?;

    // Save download to local folder and verify contents
    let download_path = format!("./test_download_{}.csv", std::process::id());
    download.save_as(&download_path).await?;

    // Read and verify the downloaded CSV file contents
    let actual_csv_content = fs::read_to_string(&download_path)?;
    let expected_csv_content = "fname,lname\nthe first,the last\nthe first2,the last2\n";
    assert_eq!(
        actual_csv_content.trim(),
        expected_csv_content.trim(),
        "Downloaded CSV content should match expected data"
    );

    // Clean up downloaded file
    if Path::new(&download_path).exists() {
        fs::remove_file(&download_path)?;
    }

    // Step 11: Test JSON download functionality
    let download_waiter = page.expect_download(Some(3000.0)).await?;
    page.locator("button:has-text('Download JSON')")
        .await
        .first()
        .click(Some(ClickOptions::builder().timeout(3000.0).build()))
        .await?;
    let download = download_waiter.wait().await?;

    // Save download to local folder and verify contents
    let download_path = format!("./test_download_{}.json", std::process::id());
    download.save_as(&download_path).await?;

    // Read and verify the downloaded JSON file contents
    let actual_json_content = fs::read_to_string(&download_path)?;
    let expected_json_content = r#"{"fields":["fname","lname"],"rows":[["the first","the last"],["the first2","the last2"]]}"#;

    // Parse both JSON strings to compare content (handles formatting differences)
    let actual_json: serde_json::Value = serde_json::from_str(&actual_json_content)?;
    let expected_json: serde_json::Value = serde_json::from_str(expected_json_content)?;
    assert_eq!(
        actual_json, expected_json,
        "Downloaded JSON content should match expected data"
    );

    // Clean up downloaded file
    if Path::new(&download_path).exists() {
        fs::remove_file(&download_path)?;
    }

    // Final verification screenshot
    BrowserHelpers::screenshot_compare(page, "database_test_complete", &[]).await?;

    Ok(())
}
