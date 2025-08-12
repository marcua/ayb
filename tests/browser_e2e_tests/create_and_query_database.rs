use crate::utils::browser::BrowserHelpers;
use playwright::api::Page;
use std::error::Error;

pub async fn test_create_and_query_database_flow(
    page: &Page,
    username: &str,
) -> Result<(), Box<dyn Error>> {
    // Step 1: Verify we're on the user dashboard and take initial screenshot
    // After profile test, the display name has changed from username to "Entity 0"
    let expected_title = "Entity 0 - ayb";
    assert_eq!(page.title().await?, expected_title);

    BrowserHelpers::screenshot_compare(&page, "dashboard_before_database_creation", &[]).await?;

    // Step 2: Click on "Create database" button
    page.click_builder("button:has-text('Create database')")
        .timeout(3000.0)
        .click()
        .await?;

    // Wait for the create database form to appear
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    // Screenshot of the create database form
    BrowserHelpers::screenshot_compare(&page, "create_database_form", &[]).await?;

    // Step 3: Fill in database name (using similar name to e2e tests)
    page.fill_builder("input[name='database_slug']", "test.sqlite")
        .timeout(1000.0)
        .fill()
        .await?;

    // Screenshot of filled form
    BrowserHelpers::screenshot_compare(&page, "database_form_filled", &[]).await?;

    // Step 4: Submit the create database form
    page.click_builder("button[type='submit']:has-text('Create database')")
        .timeout(5000.0)
        .click()
        .await?;

    // Wait for database creation and page refresh
    tokio::time::sleep(std::time::Duration::from_millis(2000)).await;

    // Screenshot after database creation
    BrowserHelpers::screenshot_compare(&page, "database_created", &[]).await?;

    // Step 5: Check if we're already on the database page or if we need to navigate there
    let current_title = page.title().await?;
    let database_page_title = format!("Explore {}/test.sqlite - ayb", username);

    if current_title != database_page_title {
        // We're still on the entity page, click on the database link
        page.click_builder("a[href$='test.sqlite']")
            .timeout(3000.0)
            .click()
            .await?;

        // Wait for database page to load
        tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
    }

    // Verify we're on the database page
    assert_eq!(page.title().await?, database_page_title);

    // Screenshot of database page
    BrowserHelpers::screenshot_compare(&page, "database_page", &[]).await?;

    // Step 6: Create the same table as in e2e tests
    let create_table_query = "CREATE TABLE test_table(fname varchar, lname varchar);";

    page.fill_builder("textarea[name='query']", create_table_query)
        .timeout(1000.0)
        .fill()
        .await?;

    // Screenshot with create table query
    BrowserHelpers::screenshot_compare(&page, "create_table_query", &[]).await?;

    // Run the create table query
    page.click_builder("button:has-text('Run query')")
        .timeout(5000.0)
        .click()
        .await?;

    // Wait for query to execute
    tokio::time::sleep(std::time::Duration::from_millis(1000)).await;

    // Screenshot after table creation
    BrowserHelpers::screenshot_compare(&page, "table_created", &[]).await?;

    // Step 7: Insert data (same as e2e tests)
    let insert_query1 =
        "INSERT INTO test_table (fname, lname) VALUES (\"the first\", \"the last\");";

    // Clear previous query and enter insert query
    page.fill_builder("textarea[name='query']", "")
        .timeout(1000.0)
        .fill()
        .await?;

    page.fill_builder("textarea[name='query']", insert_query1)
        .timeout(1000.0)
        .fill()
        .await?;

    page.click_builder("button:has-text('Run query')")
        .timeout(5000.0)
        .click()
        .await?;

    tokio::time::sleep(std::time::Duration::from_millis(1000)).await;

    // Insert second row
    let insert_query2 =
        "INSERT INTO test_table (fname, lname) VALUES (\"the first2\", \"the last2\");";

    page.fill_builder("textarea[name='query']", "")
        .timeout(1000.0)
        .fill()
        .await?;

    page.fill_builder("textarea[name='query']", insert_query2)
        .timeout(1000.0)
        .fill()
        .await?;

    page.click_builder("button:has-text('Run query')")
        .timeout(5000.0)
        .click()
        .await?;

    tokio::time::sleep(std::time::Duration::from_millis(1000)).await;

    // Screenshot after data insertion
    BrowserHelpers::screenshot_compare(&page, "data_inserted", &[]).await?;

    // Step 8: Query the data (same SELECT as e2e tests)
    let select_query = "SELECT * FROM test_table;";

    page.fill_builder("textarea[name='query']", "")
        .timeout(1000.0)
        .fill()
        .await?;

    page.fill_builder("textarea[name='query']", select_query)
        .timeout(1000.0)
        .fill()
        .await?;

    // Screenshot with select query
    BrowserHelpers::screenshot_compare(&page, "select_query", &[]).await?;

    page.click_builder("button:has-text('Run query')")
        .timeout(5000.0)
        .click()
        .await?;

    // Wait for results to load
    tokio::time::sleep(std::time::Duration::from_millis(1500)).await;

    // Screenshot of query results table
    BrowserHelpers::screenshot_compare(&page, "query_results", &[]).await?;

    // Step 9: Verify the results contain the expected data
    let page_text = page.inner_text("body", None).await?;

    // Check that the table shows the correct data
    assert!(
        page_text.contains("the first"),
        "Query results should contain 'the first'"
    );
    assert!(
        page_text.contains("the last"),
        "Query results should contain 'the last'"
    );
    assert!(
        page_text.contains("the first2"),
        "Query results should contain 'the first2'"
    );
    assert!(
        page_text.contains("the last2"),
        "Query results should contain 'the last2'"
    );

    // Verify that we have 2 rows
    assert!(
        page_text.contains("2 rows"),
        "Query results should show '2 rows'"
    );

    // Check that the table headers are correct
    assert!(
        page_text.contains("fname"),
        "Query results should contain 'fname' column header"
    );
    assert!(
        page_text.contains("lname"),
        "Query results should contain 'lname' column header"
    );

    // Step 10: Verify CSV download button exists and is clickable
    assert!(page.inner_text("body", None).await?.contains("Download CSV"), "CSV download button should be visible");
    
    // Step 11: Verify JSON download button exists and is clickable
    assert!(page.inner_text("body", None).await?.contains("Download JSON"), "JSON download button should be visible");

    // Final verification screenshot
    BrowserHelpers::screenshot_compare(&page, "database_test_complete", &[]).await?;

    Ok(())
}
