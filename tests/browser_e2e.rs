use std::error::Error;

mod utils;
use utils::browser::BrowserHelpers;
use utils::testing::{AybServer, Cleanup};

#[tokio::test]
async fn test_registration_flow() -> Result<(), Box<dyn Error>> {
    let _cleanup = Cleanup;

    // Reset database
    std::process::Command::new("tests/reset_db_browser_sqlite.sh")
        .output()
        .expect("Failed to reset database");

    // Start ayb server
    let _ayb_server = AybServer::run_browser("sqlite").expect("failed to start the ayb server");

    // Use built-in email file backend (no external SMTP server needed)

    // Give servers time to start
    tokio::time::sleep(std::time::Duration::from_secs(5)).await;

    // Initialize browser using helper method
    let (_playwright, page) = BrowserHelpers::setup_browser().await?;

    // Step 1: Navigate to registration page
    page.goto_builder("http://localhost:5433/register")
        .timeout(5000.0)
        .goto()
        .await?;

    // Verify page title
    assert_eq!(page.title().await?, "Create account - ayb");

    // Screenshot comparison of registration page
    let registration_page_matches = BrowserHelpers::screenshot_compare(
        &page,
        "registration_page",
        &[], // No elements to grey out for this test
    )
    .await?;

    if !registration_page_matches {
        println!("⚠ Registration page screenshot differs from reference");
    } else {
        println!("✓ Registration page matches reference screenshot");
    }

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
    let filled_form_matches =
        BrowserHelpers::screenshot_compare(&page, "registration_form_filled", &[]).await?;

    if !filled_form_matches {
        println!("⚠ Filled registration form screenshot differs from reference");
    } else {
        println!("✓ Filled registration form matches reference screenshot");
    }

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
    let check_email_matches =
        BrowserHelpers::screenshot_compare(&page, "check_email_page", &[]).await?;

    if !check_email_matches {
        println!("⚠ Check email page screenshot differs from reference");
    } else {
        println!("✓ Check email page matches reference screenshot");
    }

    println!("✓ Registration flow completed successfully");

    // Step 5: Extract confirmation token from email file
    let email_file = "tests/ayb_data_browser_sqlite/emails.jsonl";

    // Wait for email to arrive
    tokio::time::sleep(std::time::Duration::from_millis(1000)).await;

    let emails = BrowserHelpers::parse_email_file(email_file)?;
    let user_emails: Vec<_> = emails.into_iter().filter(|e| e.to == email).collect();
    assert!(!user_emails.is_empty(), "Should receive confirmation email");

    let confirmation_token = BrowserHelpers::extract_token_from_emails(&user_emails)
        .expect("Should extract token from email");
    let confirmation_url = format!("http://localhost:5433/confirm/{}", confirmation_token);

    println!("✓ Extracted confirmation token from email");

    // Step 6: Navigate to confirmation link
    page.goto_builder(&confirmation_url)
        .timeout(5000.0)
        .goto()
        .await?;

    // Step 7: Verify we're now on the authenticated user dashboard
    let expected_title = format!("{} - ayb", username);
    assert_eq!(page.title().await?, expected_title);

    // Screenshot comparison of user dashboard
    let dashboard_matches =
        BrowserHelpers::screenshot_compare(&page, "user_dashboard", &[]).await?;

    if !dashboard_matches {
        println!("⚠ User dashboard screenshot differs from reference");
    } else {
        println!("✓ User dashboard matches reference screenshot");
    }

    println!("✓ User {} authenticated successfully", username);

    Ok(())
}
