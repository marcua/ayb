use crate::browser_e2e_tests::test_registration_flow;
use crate::utils::browser::BrowserHelpers;
use playwright_rs::{BrowserContext, ClickOptions, FillOptions, GotoOptions, Page};
use std::error::Error;
use std::time::Duration;

pub struct UserBrowserProfile {
    pub username: String,
    pub page: Page,
}

/// Register multiple users in separate browser contexts
pub async fn register_multiple_users(
    contexts_and_pages: Vec<(BrowserContext, Page)>,
    base_url: &str,
    test_type: &str,
) -> Result<Vec<UserBrowserProfile>, Box<dyn Error>> {
    let mut users = Vec::new();

    for (i, (_context, page)) in contexts_and_pages.into_iter().enumerate() {
        let username = test_registration_flow(&page, base_url, test_type).await?;

        println!("Registered User {}: {}", i + 1, username);

        users.push(UserBrowserProfile { username, page });
    }

    Ok(users)
}

pub async fn test_permissions_flow(base_url: &str, test_type: &str) -> Result<(), Box<dyn Error>> {
    // Step 1: Set up 3 isolated browser contexts
    let (_playwright, contexts_and_pages) = BrowserHelpers::set_up_browser(3).await?;

    // Step 2: Register 3 users in separate contexts
    let mut users = register_multiple_users(contexts_and_pages, base_url, test_type).await?;

    let (user_a, rest) = users.split_at_mut(1);
    let (user_b, user_c) = rest.split_at_mut(1);
    let user_a = &mut user_a[0];
    let user_b = &mut user_b[0];
    let user_c = &mut user_c[0];

    println!(
        "All users registered: A={}, B={}, C={}",
        user_a.username, user_b.username, user_c.username
    );

    // Step 3: User A creates a database
    BrowserHelpers::screenshot_compare(&user_a.page, "userA_dashboard_before_db", &[]).await?;

    // Create database
    user_a
        .page
        .locator("button:has-text('Create database')")
        .await
        .first()
        .click(Some(ClickOptions::builder().timeout(3000.0).build()))
        .await?;

    user_a
        .page
        .locator("input[name='database_slug']")
        .await
        .first()
        .fill(
            "shared_test.sqlite",
            Some(FillOptions::builder().timeout(1000.0).build()),
        )
        .await?;

    // Database is created as private by default
    BrowserHelpers::screenshot_compare(&user_a.page, "userA_create_db_private", &[]).await?;

    user_a
        .page
        .locator("button[type='submit']:has-text('Create database')")
        .await
        .first()
        .click(Some(ClickOptions::builder().timeout(5000.0).build()))
        .await?;

    BrowserHelpers::screenshot_compare(&user_a.page, "userA_database_created", &[]).await?;

    // Step 4: User A creates the test table
    let create_table_query = "CREATE TABLE test_table(fname varchar, lname varchar);";

    user_a
        .page
        .locator("textarea[name='query']")
        .await
        .first()
        .fill(
            create_table_query,
            Some(FillOptions::builder().timeout(1000.0).build()),
        )
        .await?;

    user_a
        .page
        .locator("button:has-text('Run query')")
        .await
        .first()
        .click(Some(ClickOptions::builder().timeout(5000.0).build()))
        .await?;

    BrowserHelpers::screenshot_compare(&user_a.page, "userA_table_created", &[]).await?;

    // Step 5: Users B and C should not see User A's private database
    user_b
        .page
        .goto(
            &format!("{}/{}", base_url, user_a.username),
            Some(GotoOptions::new().timeout(Duration::from_millis(5000))),
        )
        .await?;

    let page_content_b = user_b.page.locator("body").await.inner_text().await?;
    let can_see_db_b = page_content_b.contains("shared_test.sqlite");
    assert!(
        !can_see_db_b,
        "User B should not be able to see User A's private database"
    );
    BrowserHelpers::screenshot_compare(&user_b.page, "userB_no_access_private", &[]).await?;

    user_c
        .page
        .goto(
            &format!("{}/{}", base_url, user_a.username),
            Some(GotoOptions::new().timeout(Duration::from_millis(5000))),
        )
        .await?;

    let page_content_c = user_c.page.locator("body").await.inner_text().await?;
    let can_see_db_c = page_content_c.contains("shared_test.sqlite");
    assert!(
        !can_see_db_c,
        "User C should not be able to see User A's private database"
    );
    BrowserHelpers::screenshot_compare(&user_c.page, "userC_no_access_private", &[]).await?;

    // Step 6: Test public read-only sharing
    // User A navigates to database and clicks sharing tab
    user_a
        .page
        .goto(
            &format!("{}/{}/shared_test.sqlite", base_url, user_a.username),
            Some(GotoOptions::new().timeout(Duration::from_millis(5000))),
        )
        .await?;

    user_a
        .page
        .locator("a[href='#sharing']")
        .await
        .first()
        .click(Some(ClickOptions::builder().timeout(5000.0).build()))
        .await?;

    BrowserHelpers::screenshot_compare(&user_a.page, "userA_sharing_tab", &[]).await?;

    // Set public sharing to read-only
    user_a
        .page
        .locator("button[data-value='read-only']")
        .await
        .first()
        .click(Some(ClickOptions::builder().timeout(3000.0).build()))
        .await?;

    user_a
        .page
        .locator("#update-public-sharing-btn")
        .await
        .first()
        .click(Some(ClickOptions::builder().timeout(5000.0).build()))
        .await?;

    BrowserHelpers::screenshot_compare(&user_a.page, "userA_set_public_readonly", &[]).await?;

    // Users B and C should now be able to access the database
    user_b
        .page
        .goto(
            &format!("{}/{}/shared_test.sqlite", base_url, user_a.username),
            Some(GotoOptions::new().timeout(Duration::from_millis(5000))),
        )
        .await?;

    BrowserHelpers::screenshot_compare(&user_b.page, "userB_can_access_readonly", &[]).await?;

    // User B can run read-only query
    user_b
        .page
        .locator("textarea[name='query']")
        .await
        .first()
        .fill(
            "SELECT COUNT(*) FROM test_table;",
            Some(FillOptions::builder().timeout(1000.0).build()),
        )
        .await?;

    user_b
        .page
        .locator("button:has-text('Run query')")
        .await
        .first()
        .click(Some(ClickOptions::builder().timeout(5000.0).build()))
        .await?;

    BrowserHelpers::screenshot_compare(&user_b.page, "userB_readonly_query_success", &[]).await?;

    // Verify User B can see the count result (should be 0 since no data inserted yet)
    let query_results_b_count = user_b
        .page
        .locator("#query-results")
        .await
        .inner_text()
        .await?;
    assert!(
        query_results_b_count.contains("0"),
        "User B should see count of 0 rows in empty table"
    );

    // User B cannot run insert query (should fail)
    user_b
        .page
        .locator("textarea[name='query']")
        .await
        .first()
        .fill(
            "INSERT INTO test_table (fname, lname) VALUES ('unauthorized', 'insert');",
            Some(FillOptions::builder().timeout(1000.0).build()),
        )
        .await?;

    user_b
        .page
        .locator("button:has-text('Run query')")
        .await
        .first()
        .click(Some(ClickOptions::builder().timeout(5000.0).build()))
        .await?;

    BrowserHelpers::screenshot_compare(&user_b.page, "userB_insert_query_failed", &[]).await?;

    // Verify the error message indicates read-only access
    let query_results_b = user_b
        .page
        .locator("#query-results")
        .await
        .inner_text()
        .await?;
    assert!(
        query_results_b.contains("Attempted to write to database while in read-only mode"),
        "User B should get the specific read-only error when trying to insert"
    );

    // User C should also be able to access the database now
    user_c
        .page
        .goto(
            &format!("{}/{}/shared_test.sqlite", base_url, user_a.username),
            Some(GotoOptions::new().timeout(Duration::from_millis(5000))),
        )
        .await?;

    BrowserHelpers::screenshot_compare(&user_c.page, "userC_can_access_readonly", &[]).await?;

    // Step 7: Test private database with specific user sharing
    println!("Testing private database with specific user sharing...");

    // User A sets database back to private
    user_a
        .page
        .goto(
            &format!("{}/{}/shared_test.sqlite", base_url, user_a.username),
            Some(GotoOptions::new().timeout(Duration::from_millis(5000))),
        )
        .await?;

    user_a
        .page
        .locator("a[href='#sharing']")
        .await
        .first()
        .click(Some(ClickOptions::builder().timeout(5000.0).build()))
        .await?;

    user_a
        .page
        .locator("button[data-value='no-access']")
        .await
        .first()
        .click(Some(ClickOptions::builder().timeout(3000.0).build()))
        .await?;

    user_a
        .page
        .locator("#update-public-sharing-btn")
        .await
        .first()
        .click(Some(ClickOptions::builder().timeout(5000.0).build()))
        .await?;

    BrowserHelpers::screenshot_compare(&user_a.page, "userA_set_back_to_private", &[]).await?;

    // User A shares specifically with User B
    user_a
        .page
        .locator("#share-entity")
        .await
        .first()
        .fill(
            &user_b.username,
            Some(FillOptions::builder().timeout(1000.0).build()),
        )
        .await?;

    user_a
        .page
        .locator("#entity-sharing-form button[data-value='read-only']")
        .await
        .first()
        .click(Some(ClickOptions::builder().timeout(3000.0).build()))
        .await?;

    user_a
        .page
        .locator("#share-entity-btn")
        .await
        .first()
        .click(Some(ClickOptions::builder().timeout(5000.0).build()))
        .await?;

    // Sleep a little bit so new sharing table can be retrieved/rendered.
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;

    BrowserHelpers::screenshot_compare(&user_a.page, "userA_shared_with_userB", &[]).await?;

    // User B should now be able to access the database
    user_b
        .page
        .goto(
            &format!("{}/{}/shared_test.sqlite", base_url, user_a.username),
            Some(GotoOptions::new().timeout(Duration::from_millis(5000))),
        )
        .await?;

    BrowserHelpers::screenshot_compare(&user_b.page, "userB_specific_access_granted", &[]).await?;

    // User B can run read-only query
    user_b
        .page
        .locator("textarea[name='query']")
        .await
        .first()
        .fill(
            "SELECT COUNT(*) FROM test_table;",
            Some(FillOptions::builder().timeout(1000.0).build()),
        )
        .await?;

    user_b
        .page
        .locator("button:has-text('Run query')")
        .await
        .first()
        .click(Some(ClickOptions::builder().timeout(5000.0).build()))
        .await?;

    BrowserHelpers::screenshot_compare(&user_b.page, "userB_specific_query_success", &[]).await?;

    // Verify User B can see the count result (should still be 0)
    let query_results_b_specific = user_b
        .page
        .locator("#query-results")
        .await
        .inner_text()
        .await?;
    assert!(
        query_results_b_specific.contains("0"),
        "User B should see count of 0 rows when specifically shared"
    );

    // User C should still not be able to access the database
    user_c
        .page
        .goto(
            &format!("{}/{}/shared_test.sqlite", base_url, user_a.username),
            Some(GotoOptions::new().timeout(Duration::from_millis(5000))),
        )
        .await?;

    let page_content_c_final = user_c.page.locator("body").await.inner_text().await?;
    let can_see_db_c_final = page_content_c_final.contains("shared_test.sqlite")
        || page_content_c_final.contains("Query");
    assert!(
        !can_see_db_c_final,
        "User C should still not be able to see User A's database after specific sharing with User B"
    );

    BrowserHelpers::screenshot_compare(&user_c.page, "userC_still_no_access", &[]).await?;

    Ok(())
}
