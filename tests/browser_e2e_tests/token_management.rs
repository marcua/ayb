use crate::utils::browser::BrowserHelpers;
use playwright::api::Page;
use std::error::Error;

// TODO(marcua): Once the OAuth flow is implemented, add tests for scoped tokens
// that reduce the access level a user would otherwise have (e.g., a read-only
// token for a user with read-write access). This will exercise the
// highest_query_access_level permission capping logic.

pub async fn test_token_management_flow(page: &Page, username: &str) -> Result<(), Box<dyn Error>> {
    // Step 1: Navigate to the tokens page via the dropdown menu
    // Click on the username dropdown to open the menu
    page.click_builder(&format!("a:has-text('{}')", username))
        .timeout(5000.0)
        .click()
        .await?;

    // Take screenshot of the dropdown menu
    BrowserHelpers::screenshot_compare(page, "tokens_dropdown_menu", &[]).await?;

    // Click on "Tokens" in the dropdown
    page.click_builder("a:has-text('Tokens')")
        .timeout(5000.0)
        .click()
        .await?;

    // Step 2: Verify we're on the tokens page
    let page_url = page.url()?;
    assert!(
        page_url.contains("/settings/tokens"),
        "Should be on the tokens page, got: {}",
        page_url
    );

    // Take screenshot of the tokens page
    BrowserHelpers::screenshot_compare(page, "tokens_page_initial", &[]).await?;

    // Step 3: Verify the page content
    let page_text = page.inner_text("body", None).await?;
    assert!(
        page_text.contains("API Tokens") || page_text.contains("Short token"),
        "Tokens page should show token-related content"
    );

    // Step 4: Check for breadcrumbs
    assert!(
        page_text.contains(username),
        "Breadcrumbs should contain the username"
    );

    // Step 5: Check if there's at least one token in the table
    // (since we already have API keys from registration)
    let table_exists = page.query_selector("table").await?.is_some();
    assert!(table_exists, "Token table should exist on the page");

    // Step 6: Count initial token rows and verify revoke buttons exist
    let initial_rows = page.query_selector_all("table tbody tr").await?;
    let initial_count = initial_rows.len();
    assert!(
        initial_count >= 2,
        "Should have at least 2 tokens (we need one to revoke and one to keep using)"
    );

    // Get all revoke buttons and click the last one (not the first/currently used token)
    let revoke_buttons = page.query_selector_all("button:has-text('Revoke')").await?;
    assert!(
        !revoke_buttons.is_empty(),
        "Should have at least one revoke button"
    );
    BrowserHelpers::screenshot_compare(page, "tokens_page_with_revoke_button", &[]).await?;

    // Click the last revoke button (safest - least likely to be the active session token)
    // Override window.confirm to auto-accept the hx-confirm dialog
    page.evaluate::<(), ()>("window.confirm = () => true", ())
        .await?;

    let last_button = revoke_buttons
        .last()
        .expect("Should have at least one button");
    last_button.click_builder().click().await?;

    // Wait for htmx to process the deletion
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    // Take screenshot after revocation
    BrowserHelpers::screenshot_compare(page, "tokens_page_after_revoke", &[]).await?;

    // Verify token count decreased (htmx should have removed the row)
    let rows_after_revoke = page.query_selector_all("table tbody tr").await?;
    assert_eq!(
        rows_after_revoke.len(),
        initial_count - 1,
        "Should have one less token after revocation"
    );

    // Reload the page and verify the revoked token is still gone
    page.reload_builder().reload().await?;
    page.wait_for_selector_builder("table")
        .timeout(5000.0)
        .wait_for_selector()
        .await?;

    let rows_after_reload = page.query_selector_all("table tbody tr").await?;
    assert_eq!(
        rows_after_reload.len(),
        initial_count - 1,
        "Token should still be gone after page reload"
    );
    BrowserHelpers::screenshot_compare(page, "tokens_page_after_reload", &[]).await?;

    // Step 7: Navigate back to profile to verify navigation works
    page.click_builder(&format!("a:has-text('{}')", username))
        .timeout(5000.0)
        .click()
        .await?;

    page.click_builder("a:has-text('Profile')")
        .timeout(5000.0)
        .click()
        .await?;

    // Verify we're back on the profile page
    let profile_url = page.url()?;
    assert!(
        profile_url.ends_with(&format!("/{}", username))
            || profile_url.contains(&format!("/{}/", username)),
        "Should be on profile page, got: {}",
        profile_url
    );

    BrowserHelpers::screenshot_compare(page, "tokens_navigation_complete", &[]).await?;

    Ok(())
}
