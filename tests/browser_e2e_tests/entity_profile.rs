use crate::utils::browser::BrowserHelpers;
use playwright_rs::{ClickOptions, FillOptions, GotoOptions, Page};
use std::error::Error;
use std::time::Duration;

pub async fn test_entity_profile_flow(page: &Page, username: &str) -> Result<(), Box<dyn Error>> {
    // Step 1: Verify we're on the entity dashboard
    let expected_title = format!("{} - ayb", username);
    assert_eq!(page.title().await?, expected_title);

    // Take initial screenshot of the dashboard
    BrowserHelpers::screenshot_compare(page, "entity_dashboard_reference", &[]).await?;

    // Step 2: Enter profile edit mode
    page.locator("button:has-text('Edit profile')")
        .await
        .first()
        .click(Some(ClickOptions::builder().timeout(5000.0).build()))
        .await?;

    // Screenshot of edit mode activated
    BrowserHelpers::screenshot_compare(page, "profile_edit_mode", &[]).await?;

    // Step 3: Fill in profile fields with test data
    page.locator("input[name='display_name']")
        .await
        .first()
        .fill(
            "Entity 0",
            Some(FillOptions::builder().timeout(1000.0).build()),
        )
        .await?;

    page.locator("input[name='description']")
        .await
        .first()
        .fill(
            "Entity 0 NEW description",
            Some(FillOptions::builder().timeout(1000.0).build()),
        )
        .await?;

    page.locator("input[name='organization']")
        .await
        .first()
        .fill(
            "Entity 0 organization",
            Some(FillOptions::builder().timeout(1000.0).build()),
        )
        .await?;

    page.locator("input[name='location']")
        .await
        .first()
        .fill("NYC", Some(FillOptions::builder().timeout(1000.0).build()))
        .await?;

    // Add first link by clicking the "Add link" button and filling the field
    page.locator("button:has-text('Add link')")
        .await
        .first()
        .click(Some(ClickOptions::builder().timeout(2000.0).build()))
        .await?;

    // Fill the first link input (should be the only one at this point)
    page.locator("input[name='links[]']")
        .await
        .first()
        .fill(
            "http://ayb.host/",
            Some(FillOptions::builder().timeout(1000.0).build()),
        )
        .await?;

    // Add a second link
    page.locator("button:has-text('Add link')")
        .await
        .first()
        .click(Some(ClickOptions::builder().timeout(2000.0).build()))
        .await?;

    page.locator("div.link-input-group:nth-child(2) input[name='links[]']")
        .await
        .first()
        .fill(
            "http://ayb2.host/",
            Some(FillOptions::builder().timeout(3000.0).build()),
        )
        .await?;

    // Screenshot of filled form
    BrowserHelpers::screenshot_compare(page, "profile_form_filled", &[]).await?;

    // Step 4: Save the profile
    page.locator("button:has-text('Save')")
        .await
        .first()
        .click(Some(ClickOptions::builder().timeout(5000.0).build()))
        .await?;

    // Screenshot after saving
    BrowserHelpers::screenshot_compare(page, "profile_saved", &[]).await?;

    // Step 5: Reload the page to ensure data persistence
    page.reload(Some(
        GotoOptions::new().timeout(Duration::from_millis(5000)),
    ))
    .await?;

    // Screenshot after reload to confirm data persisted
    BrowserHelpers::screenshot_compare(page, "profile_after_reload", &[]).await?;

    // Step 6: Verify the profile data was saved correctly by checking visible text

    // Check display name
    let page_text = page.locator("body").await.inner_text().await?;
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
    BrowserHelpers::screenshot_compare(page, "profile_verification_complete", &[]).await?;

    Ok(())
}
