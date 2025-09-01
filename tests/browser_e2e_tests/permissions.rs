use crate::browser_e2e_tests::test_registration_flow;
use crate::utils::browser::BrowserHelpers;
use playwright::api::Page;
use std::error::Error;

pub struct UserBrowserProfile {
    pub username: String,
    pub page: Page,
}

/// Register multiple users in separate browser contexts
pub async fn register_multiple_users(
    contexts_and_pages: Vec<(playwright::api::BrowserContext, Page)>,
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
        .click_builder("button:has-text('Create database')")
        .timeout(3000.0)
        .click()
        .await?;

    user_a
        .page
        .fill_builder("input[name='database_slug']", "shared_test.sqlite")
        .timeout(1000.0)
        .fill()
        .await?;

    // Database is created as private by default
    BrowserHelpers::screenshot_compare(&user_a.page, "userA_create_db_private", &[]).await?;

    user_a
        .page
        .click_builder("button[type='submit']:has-text('Create database')")
        .timeout(5000.0)
        .click()
        .await?;

    BrowserHelpers::screenshot_compare(&user_a.page, "userA_database_created", &[]).await?;

    // Step 4: User A creates the test table
    let create_table_query = "CREATE TABLE test_table(fname varchar, lname varchar);";

    user_a
        .page
        .fill_builder("textarea[name='query']", create_table_query)
        .timeout(1000.0)
        .fill()
        .await?;

    user_a
        .page
        .click_builder("button:has-text('Run query')")
        .timeout(5000.0)
        .click()
        .await?;

    BrowserHelpers::screenshot_compare(&user_a.page, "userA_table_created", &[]).await?;

    // Step 5: Users B and C should not see User A's private database
    user_b
        .page
        .goto_builder(&format!("{}/{}", base_url, user_a.username))
        .timeout(5000.0)
        .goto()
        .await?;

    let page_content_b = user_b.page.inner_text("body", None).await?;
    let can_see_db_b = page_content_b.contains("shared_test.sqlite");
    assert!(
        !can_see_db_b,
        "User B should not be able to see User A's private database"
    );
    BrowserHelpers::screenshot_compare(&user_b.page, "userB_no_access_private", &[]).await?;

    user_c
        .page
        .goto_builder(&format!("{}/{}", base_url, user_a.username))
        .timeout(5000.0)
        .goto()
        .await?;

    let page_content_c = user_c.page.inner_text("body", None).await?;
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
        .goto_builder(&format!(
            "{}/{}/shared_test.sqlite",
            base_url, user_a.username
        ))
        .timeout(5000.0)
        .goto()
        .await?;

    user_a
        .page
        .click_builder("a[href='#sharing']")
        .timeout(5000.0)
        .click()
        .await?;

    BrowserHelpers::screenshot_compare(&user_a.page, "userA_sharing_tab", &[]).await?;

    // Set public sharing to read-only
    user_a
        .page
        .click_builder("button[data-value='read-only']")
        .timeout(3000.0)
        .click()
        .await?;

    user_a
        .page
        .click_builder("#update-public-sharing-btn")
        .timeout(5000.0)
        .click()
        .await?;

    BrowserHelpers::screenshot_compare(&user_a.page, "userA_set_public_readonly", &[]).await?;

    // Users B and C should now be able to access the database
    user_b
        .page
        .goto_builder(&format!(
            "{}/{}/shared_test.sqlite",
            base_url, user_a.username
        ))
        .timeout(5000.0)
        .goto()
        .await?;

    BrowserHelpers::screenshot_compare(&user_b.page, "userB_can_access_readonly", &[]).await?;

    // User B can run read-only query
    user_b
        .page
        .fill_builder("textarea[name='query']", "SELECT COUNT(*) FROM test_table;")
        .timeout(1000.0)
        .fill()
        .await?;

    user_b
        .page
        .click_builder("button:has-text('Run query')")
        .timeout(5000.0)
        .click()
        .await?;

    BrowserHelpers::screenshot_compare(&user_b.page, "userB_readonly_query_success", &[]).await?;

    // Verify User B can see the count result (should be 0 since no data inserted yet)
    let query_results_b_count = user_b.page.inner_text("#query-results", None).await?;
    assert!(
        query_results_b_count.contains("0"),
        "User B should see count of 0 rows in empty table"
    );

    // User B cannot run insert query (should fail)
    user_b
        .page
        .fill_builder(
            "textarea[name='query']",
            "INSERT INTO test_table (fname, lname) VALUES ('unauthorized', 'insert');",
        )
        .timeout(1000.0)
        .fill()
        .await?;

    user_b
        .page
        .click_builder("button:has-text('Run query')")
        .timeout(5000.0)
        .click()
        .await?;

    BrowserHelpers::screenshot_compare(&user_b.page, "userB_insert_query_failed", &[]).await?;

    // Verify the error message indicates read-only access
    let query_results_b = user_b.page.inner_text("#query-results", None).await?;
    assert!(
        query_results_b.contains("Attempted to write to database while in read-only mode"),
        "User B should get the specific read-only error when trying to insert"
    );

    // User C should also be able to access the database now
    user_c
        .page
        .goto_builder(&format!(
            "{}/{}/shared_test.sqlite",
            base_url, user_a.username
        ))
        .timeout(5000.0)
        .goto()
        .await?;

    BrowserHelpers::screenshot_compare(&user_c.page, "userC_can_access_readonly", &[]).await?;

    // Step 7: Test private database with specific user sharing
    println!("Testing private database with specific user sharing...");

    // User A sets database back to private
    user_a
        .page
        .goto_builder(&format!(
            "{}/{}/shared_test.sqlite",
            base_url, user_a.username
        ))
        .timeout(5000.0)
        .goto()
        .await?;

    user_a
        .page
        .click_builder("a[href='#sharing']")
        .timeout(5000.0)
        .click()
        .await?;

    user_a
        .page
        .click_builder("button[data-value='no-access']")
        .timeout(3000.0)
        .click()
        .await?;

    user_a
        .page
        .click_builder("#update-public-sharing-btn")
        .timeout(5000.0)
        .click()
        .await?;

    BrowserHelpers::screenshot_compare(&user_a.page, "userA_set_back_to_private", &[]).await?;

    // User A shares specifically with User B
    user_a
        .page
        .fill_builder("#share-entity", &user_b.username)
        .timeout(1000.0)
        .fill()
        .await?;

    user_a
        .page
        .click_builder("#entity-sharing-form button[data-value='read-only']")
        .timeout(3000.0)
        .click()
        .await?;

    user_a
        .page
        .click_builder("#share-entity-btn")
        .timeout(5000.0)
        .click()
        .await?;

    BrowserHelpers::screenshot_compare(&user_a.page, "userA_shared_with_userB", &[]).await?;

    // User B should now be able to access the database
    user_b
        .page
        .goto_builder(&format!(
            "{}/{}/shared_test.sqlite",
            base_url, user_a.username
        ))
        .timeout(5000.0)
        .goto()
        .await?;

    BrowserHelpers::screenshot_compare(&user_b.page, "userB_specific_access_granted", &[]).await?;

    // User B can run read-only query
    user_b
        .page
        .fill_builder("textarea[name='query']", "SELECT COUNT(*) FROM test_table;")
        .timeout(1000.0)
        .fill()
        .await?;

    user_b
        .page
        .click_builder("button:has-text('Run query')")
        .timeout(5000.0)
        .click()
        .await?;

    BrowserHelpers::screenshot_compare(&user_b.page, "userB_specific_query_success", &[]).await?;

    // Verify User B can see the count result (should still be 0)
    let query_results_b_specific = user_b.page.inner_text("#query-results", None).await?;
    assert!(
        query_results_b_specific.contains("0"),
        "User B should see count of 0 rows when specifically shared"
    );

    // User C should still not be able to access the database
    user_c
        .page
        .goto_builder(&format!(
            "{}/{}/shared_test.sqlite",
            base_url, user_a.username
        ))
        .timeout(5000.0)
        .goto()
        .await?;

    let page_content_c_final = user_c.page.inner_text("body", None).await?;
    let can_see_db_c_final = page_content_c_final.contains("shared_test.sqlite")
        || page_content_c_final.contains("Query");
    assert!(
        !can_see_db_c_final,
        "User C should still not be able to see User A's database after specific sharing with User B"
    );

    BrowserHelpers::screenshot_compare(&user_c.page, "userC_still_no_access", &[]).await?;

    Ok(())
}
