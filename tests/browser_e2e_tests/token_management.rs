use crate::utils::browser::BrowserHelpers;
use playwright::api::Page;
use std::error::Error;

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
        page_url.contains("/-/tokens"),
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

    // Step 6: If there are revoke buttons, take a screenshot showing the actions
    let revoke_button = page.query_selector("button:has-text('Revoke')").await?;
    if revoke_button.is_some() {
        BrowserHelpers::screenshot_compare(page, "tokens_page_with_revoke_button", &[]).await?;
    }

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
