use crate::utils::ayb::{confirm, log_in};
use crate::utils::browser::BrowserHelpers;
use crate::utils::email::{extract_token_from_emails, get_emails_for_recipient};
use playwright::api::Page;
use std::error::Error;

// TODO(marcua): Once the OAuth flow is implemented, add tests for scoped tokens
// that reduce the access level a user would otherwise have (e.g., a read-only
// token for a user with read-write access). This will exercise the
// highest_query_access_level permission capping logic.

pub async fn test_token_management_flow(
    page: &Page,
    username: &str,
    base_url: &str,
    test_type: &str,
) -> Result<(), Box<dyn Error>> {
    // Create an additional token via CLI so we have 2 tokens for revocation testing
    let email = format!("{username}@example.com");
    let emails_before = get_emails_for_recipient(test_type, &email)?;
    log_in(
        base_url,
        username,
        &format!("Check your email to finish logging in {username}"),
    )?;
    let emails_after = get_emails_for_recipient(test_type, &email)?;
    assert!(
        emails_after.len() > emails_before.len(),
        "Should have received a new email after log_in"
    );
    let token = extract_token_from_emails(&[emails_after.last().unwrap().clone()])
        .expect("Should be able to extract token from email");
    confirm(base_url, &token, "Successfully authenticated")?;

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
    // We expect exactly 2 tokens: one from initial registration, one created above
    let initial_rows = page.query_selector_all("table tbody tr").await?;
    let initial_count = initial_rows.len();
    assert_eq!(
        initial_count, 2,
        "Should have exactly 2 tokens (one from registration, one created above)"
    );

    // Get all revoke buttons in the table (not including the modal's confirm button)
    let revoke_buttons = page
        .query_selector_all("#tokens-table button:has-text('Revoke')")
        .await?;
    assert_eq!(
        revoke_buttons.len(),
        2,
        "Should have exactly 2 revoke buttons in the table (one per token)"
    );
    BrowserHelpers::screenshot_compare(page, "tokens_page_with_revoke_button", &[]).await?;

    // Click the first revoke button to open the confirmation modal
    // We revoke the first token (the CLI-created one), not the browser session's token,
    // so that the browser can still authenticate after page reload.
    let first_button = revoke_buttons
        .first()
        .expect("Should have at least one button");
    first_button.click_builder().click().await?;

    // Wait for the modal to appear
    page.wait_for_selector_builder("#revoke-token-modal")
        .state(playwright::api::frame::FrameState::Visible)
        .timeout(5000.0)
        .wait_for_selector()
        .await?;

    // Extra delay for modal animation to complete
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;

    // Take screenshot of the confirmation modal
    BrowserHelpers::screenshot_compare(page, "tokens_revoke_modal", &[]).await?;

    // Click the confirm button in the modal
    page.click_builder("#confirm-revoke-btn")
        .timeout(5000.0)
        .click()
        .await?;

    // Wait for the modal to close and the row to be replaced
    tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;

    // Take screenshot after revocation - row should show "revoked successfully" message
    BrowserHelpers::screenshot_compare(page, "tokens_page_after_revoke", &[]).await?;

    // Verify the revocation message appears
    let page_text = page.inner_text("body", None).await?;
    assert!(
        page_text.contains("revoked successfully"),
        "Should show revocation success message"
    );

    // Reload the page and verify the revoked token is gone
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
