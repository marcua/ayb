use crate::email_helpers::{extract_token_from_emails, parse_email_file};
use crate::utils::browser::BrowserHelpers;
use playwright::api::Page;
use std::error::Error;

pub async fn test_registration_flow(page: &Page, base_url: &str) -> Result<(), Box<dyn Error>> {
    // Step 1: Navigate to registration page
    page.goto_builder(&format!("{}/register", base_url))
        .timeout(5000.0)
        .goto()
        .await?;

    // Verify page title
    assert_eq!(page.title().await?, "Create account - ayb");

    // Screenshot comparison of registration page
    BrowserHelpers::screenshot_compare(
        &page,
        "registration_page",
        &[], // No elements to grey out for this test
    )
    .await?;

    // Step 2: Fill registration form with unique username
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let username = format!("testuser_{}", timestamp);
    let email = format!("{}@example.com", username);

    page.fill_builder("input[name='username']", &username)
        .timeout(1000.0)
        .fill()
        .await?;
    page.fill_builder("input[name='email']", &email)
        .timeout(1000.0)
        .fill()
        .await?;

    // Screenshot of filled form
    BrowserHelpers::screenshot_compare(&page, "registration_form_filled", &[]).await?;

    // Step 3: Submit form
    page.click_builder("button:has-text('Create account')")
        .timeout(5000.0)
        .click()
        .await?;

    // Wait a moment for the page to change
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    // Step 4: Verify we're on the check email page
    assert_eq!(page.title().await?, "Check email - ayb");

    // Screenshot comparison of check email page
    BrowserHelpers::screenshot_compare(&page, "check_email_page", &[]).await?;

    // Step 5: Extract confirmation token from email file
    let email_file = "tests/ayb_data_browser_sqlite/emails.jsonl";

    // Wait for email to arrive
    tokio::time::sleep(std::time::Duration::from_millis(1000)).await;

    let emails = parse_email_file(email_file)?;
    let user_emails: Vec<_> = emails.into_iter().filter(|e| e.to == email).collect();
    assert!(!user_emails.is_empty(), "Should receive confirmation email");

    let confirmation_token =
        extract_token_from_emails(&user_emails).expect("Should extract token from email");
    let confirmation_url = format!("{}/confirm/{}", base_url, confirmation_token);

    // Step 6: Navigate to confirmation link
    page.goto_builder(&confirmation_url)
        .timeout(5000.0)
        .goto()
        .await?;

    // Wait for page to stabilize after navigation
    tokio::time::sleep(std::time::Duration::from_millis(1000)).await;

    // Step 7: Verify we're now on the authenticated user dashboard
    let expected_title = format!("{} - ayb", username);
    assert_eq!(page.title().await?, expected_title);

    // Screenshot comparison of user dashboard
    BrowserHelpers::screenshot_compare(&page, "user_dashboard", &[]).await?;

    Ok(())
}
