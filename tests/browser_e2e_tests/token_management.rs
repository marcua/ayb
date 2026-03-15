use crate::utils::browser::BrowserHelpers;
use playwright::api::Page;
use std::error::Error;

/// Test the token management UI flow.
///
/// We expect 3 tokens in the UI:
/// - 1 from initial registration (the browser session token)
/// - 2 from OAuth flow (read-only and read-write scoped tokens)
///
/// We revoke the OAuth read-only token and verify it no longer works.
pub async fn test_token_management_flow(
    page: &Page,
    username: &str,
    base_url: &str,
    oauth_token: String,
) -> Result<(), Box<dyn Error>> {
    let client = reqwest::Client::new();
    let database_path = format!("{}/test.sqlite", username);

    // Verify the OAuth token works before we revoke it
    let pre_revoke_response = client
        .post(&format!("{}/v1/{}/query", base_url, database_path))
        .header("Authorization", format!("Bearer {}", oauth_token))
        .body("SELECT 1")
        .send()
        .await?;

    assert_eq!(
        pre_revoke_response.status(),
        200,
        "OAuth token should work before revocation"
    );
    println!("Confirmed: OAuth token works before revocation");

    // Step 1: Navigate to the user's profile page first (previous test may
    // have left us on a non-ayb page like the OAuth callback URL).
    page.goto_builder(&format!("{}/{}", base_url, username))
        .goto()
        .await?;

    // Navigate to the tokens page via the dropdown menu
    page.click_builder(&format!("a:has-text('{}')", username))
        .timeout(5000.0)
        .click()
        .await?;

    BrowserHelpers::screenshot_compare(page, "tokens_dropdown_menu", &[]).await?;

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

    // Step 5: Check token table exists
    let table_exists = page.query_selector("table").await?.is_some();
    assert!(table_exists, "Token table should exist on the page");

    // Step 6: Count initial token rows and verify revoke buttons exist
    // 1 (registration) + 2 (OAuth) = 3 tokens
    let initial_rows = page.query_selector_all("table tbody tr").await?;
    let initial_count = initial_rows.len();

    assert_eq!(initial_count, 3, "Should have exactly 3 tokens");

    let revoke_buttons = page
        .query_selector_all("#tokens-table button:has-text('Revoke')")
        .await?;
    assert_eq!(
        revoke_buttons.len(),
        3,
        "Should have exactly 3 revoke buttons in the table"
    );
    BrowserHelpers::screenshot_compare(page, "tokens_page_with_revoke_buttons", &[]).await?;

    // Step 7: Revoke the read-only OAuth token by finding its row via the app name
    page.click_builder("tr:has-text('Test OAuth App (read-only)') button:has-text('Revoke')")
        .timeout(5000.0)
        .click()
        .await?;

    // Wait for the modal to appear
    page.wait_for_selector_builder("#revoke-token-modal")
        .state(playwright::api::frame::FrameState::Visible)
        .timeout(5000.0)
        .wait_for_selector()
        .await?;

    tokio::time::sleep(std::time::Duration::from_millis(200)).await;

    BrowserHelpers::screenshot_compare(page, "tokens_revoke_modal", &[]).await?;

    // Click the confirm button
    page.click_builder("#confirm-revoke-btn")
        .timeout(5000.0)
        .click()
        .await?;

    tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;

    BrowserHelpers::screenshot_compare(page, "tokens_page_after_revoke", &[]).await?;

    // Verify the revocation message appears
    let page_text = page.inner_text("body", None).await?;
    assert!(
        page_text.contains("revoked successfully"),
        "Should show revocation success message"
    );

    // Reload and verify token count decreased
    page.reload_builder().reload().await?;
    page.wait_for_selector_builder("table")
        .timeout(5000.0)
        .wait_for_selector()
        .await?;

    let rows_after_reload = page.query_selector_all("table tbody tr").await?;
    assert_eq!(
        rows_after_reload.len(),
        initial_count - 1,
        "Token should be gone after page reload"
    );
    BrowserHelpers::screenshot_compare(page, "tokens_page_after_reload", &[]).await?;

    // Step 8: Verify the revoked OAuth token no longer works.
    // Brief pause to avoid transient connection errors during snapshot cycles.
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    let response = client
        .post(&format!("{}/v1/{}/query", base_url, database_path))
        .header("Authorization", format!("Bearer {}", oauth_token))
        .body("SELECT 1")
        .send()
        .await?;

    assert_ne!(
        response.status(),
        200,
        "Revoked token should not be able to query database"
    );

    let error_body = response.text().await?;
    assert!(
        error_body.contains("revoked"),
        "Error should mention token was revoked: {}",
        error_body
    );

    println!("Confirmed: revoked OAuth token no longer works");

    // Step 9: Navigate back to profile
    page.click_builder(&format!("a:has-text('{}')", username))
        .timeout(5000.0)
        .click()
        .await?;

    page.click_builder("a:has-text('Profile')")
        .timeout(5000.0)
        .click()
        .await?;

    let profile_url = page.url()?;
    assert!(
        profile_url.ends_with(&format!("/{}", username))
            || profile_url.contains(&format!("/{}/", username)),
        "Should be on profile page, got: {}",
        profile_url
    );

    BrowserHelpers::screenshot_compare(page, "tokens_navigation_complete", &[]).await?;

    println!("Token management tests passed successfully");

    Ok(())
}
