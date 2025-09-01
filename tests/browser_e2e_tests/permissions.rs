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

    // Step 4: Users B and C should not see User A's private database
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

    // Step 5: Isolation verified by assertions above

    // Final verification screenshot
    BrowserHelpers::screenshot_compare(&user_a.page, "permissions_test_complete", &[]).await?;

    Ok(())
}
