use crate::utils::browser::BrowserHelpers;
use playwright::api::Page;
use std::error::Error;

pub async fn test_entity_profile_flow(page: &Page, username: &str) -> Result<(), Box<dyn Error>> {
    // Step 1: Verify we're on the entity dashboard
    let expected_title = format!("{} - ayb", username);
    assert_eq!(page.title().await?, expected_title);

    // Take initial screenshot of the dashboard
    BrowserHelpers::screenshot_compare(&page, "entity_dashboard_reference", &[]).await?;

    // Step 2: Enter profile edit mode
    page.click_builder("button:has-text('Edit profile')")
        .timeout(5000.0)
        .click()
        .await?;

    // Screenshot of edit mode activated
    BrowserHelpers::screenshot_compare(&page, "profile_edit_mode", &[]).await?;

    // Step 3: Fill in profile fields with test data
    page.fill_builder("input[name='display_name']", "Entity 0")
        .timeout(1000.0)
        .fill()
        .await?;

    page.fill_builder("input[name='description']", "Entity 0 NEW description")
        .timeout(1000.0)
        .fill()
        .await?;

    page.fill_builder("input[name='organization']", "Entity 0 organization")
        .timeout(1000.0)
        .fill()
        .await?;

    page.fill_builder("input[name='location']", "NYC")
        .timeout(1000.0)
        .fill()
        .await?;

    // Add first link by clicking the "Add link" button and filling the field
    page.click_builder("button:has-text('Add link')")
        .timeout(2000.0)
        .click()
        .await?;

    // Fill the first link input (should be the only one at this point)
    page.fill_builder("input[name='links[]']", "http://ayb.host/")
        .timeout(1000.0)
        .fill()
        .await?;

    // Add a second link
    page.click_builder("button:has-text('Add link')")
        .timeout(2000.0)
        .click()
        .await?;

    page.fill_builder(
        "div.link-input-group:nth-child(2) input[name='links[]']",
        "http://ayb2.host/",
    )
    .timeout(3000.0)
    .fill()
    .await?;

    // Screenshot of filled form
    BrowserHelpers::screenshot_compare(&page, "profile_form_filled", &[]).await?;

    // Step 4: Save the profile
    page.click_builder("button:has-text('Save')")
        .timeout(5000.0)
        .click()
        .await?;

    // Screenshot after saving
    BrowserHelpers::screenshot_compare(&page, "profile_saved", &[]).await?;

    // Step 5: Reload the page to ensure data persistence
    page.reload_builder().timeout(5000.0).reload().await?;

    // Screenshot after reload to confirm data persisted
    BrowserHelpers::screenshot_compare(&page, "profile_after_reload", &[]).await?;

    // Step 6: Verify the profile data was saved correctly by checking visible text

    // Check display name
    let page_text = page.inner_text("body", None).await?;
    assert!(
        page_text.contains("Entity 0"),
        "Display name should be visible after reload"
    );

    // Check description
    assert!(
        page_text.contains("Entity 0 NEW description"),
        "Description should be visible after reload"
    );

    // Check organization
    assert!(
        page_text.contains("Entity 0 organization"),
        "Organization should be visible after reload"
    );

    // Check location
    assert!(
        page_text.contains("NYC"),
        "Location should be visible after reload"
    );

    // Check links (being flexible with trailing slashes)
    assert!(
        page_text.contains("http://ayb.host") || page_text.contains("ayb.host"),
        "First link should be visible after reload"
    );
    assert!(
        page_text.contains("http://ayb2.host") || page_text.contains("ayb2.host"),
        "Second link should be visible after reload"
    );

    // Final verification screenshot
    BrowserHelpers::screenshot_compare(&page, "profile_verification_complete", &[]).await?;

    Ok(())
}
