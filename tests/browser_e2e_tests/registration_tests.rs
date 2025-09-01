use crate::utils::browser::BrowserHelpers;
use crate::utils::email::{extract_token_from_emails, get_emails_for_recipient};
use playwright::api::Page;
use std::error::Error;

pub async fn test_registration_flow(
    page: &Page,
    base_url: &str,
    test_type: &str,
) -> Result<String, Box<dyn Error>> {
    // Step 1: Navigate to registration page
    page.goto_builder(&format!("{}/register", base_url))
        .timeout(5000.0)
        .goto()
        .await?;

    // Verify page title
    assert_eq!(page.title().await?, "Create account - ayb");

    // Screenshot comparison of registration page
    BrowserHelpers::screenshot_compare(&page, "registration_page", &[]).await?;

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

    // Screenshot comparison of check email page
    BrowserHelpers::screenshot_compare(&page, "check_email_page", &[]).await?;

    // Step 4: Verify we're on the check email page
    assert_eq!(page.title().await?, "Check email - ayb");

    // Step 5: Extract confirmation token from email file
    let user_emails = get_emails_for_recipient(test_type, &email)?;
    assert!(!user_emails.is_empty(), "Should receive confirmation email");

    let confirmation_token =
        extract_token_from_emails(&user_emails).expect("Should extract token from email");
    let confirmation_url = format!("{}/confirm/{}", base_url, confirmation_token);

    // Step 6: Navigate to confirmation link
    page.goto_builder(&confirmation_url)
        .timeout(5000.0)
        .goto()
        .await?;

    // Screenshot comparison of user dashboard
    BrowserHelpers::screenshot_compare(&page, "user_dashboard", &[]).await?;

    // Step 7: Verify we're now on the authenticated user dashboard
    let expected_title = format!("{} - ayb", username);
    assert_eq!(page.title().await?, expected_title);

    Ok(username)
}
