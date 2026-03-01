use crate::utils::browser::BrowserHelpers;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use playwright::api::Page;
use sha2::{Digest, Sha256};
use std::error::Error;

/// Generate a PKCE code verifier and challenge pair
fn generate_pkce() -> (String, String) {
    // Generate a random code verifier (43-128 characters)
    let verifier: String = (0..64)
        .map(|_| {
            let idx = rand::random::<u8>() % 62;
            let c = if idx < 10 {
                (b'0' + idx) as char
            } else if idx < 36 {
                (b'a' + idx - 10) as char
            } else {
                (b'A' + idx - 36) as char
            };
            c
        })
        .collect();

    // Compute SHA256 hash and base64url encode it
    let mut hasher = Sha256::new();
    hasher.update(verifier.as_bytes());
    let hash = hasher.finalize();
    let challenge = URL_SAFE_NO_PAD.encode(hash);

    (verifier, challenge)
}

/// Test the complete OAuth flow including scoped token permission capping.
///
/// This test verifies that:
/// 1. A user can authorize an OAuth request for read-only access
/// 2. The returned token can be used to read from the database
/// 3. The returned token CANNOT be used to write to the database (permission capping)
pub async fn test_oauth_flow(
    page: &Page,
    username: &str,
    base_url: &str,
) -> Result<(), Box<dyn Error>> {
    // Generate PKCE challenge
    let (code_verifier, code_challenge) = generate_pkce();
    let state = "test_state_12345";
    let app_name = "Test OAuth App";
    let redirect_uri = format!("{}/oauth/callback", base_url);

    // Step 1: Navigate to OAuth authorize endpoint requesting read-only access
    let authorize_url = format!(
        "{}/oauth/authorize?response_type=code&redirect_uri={}&scope=read-only&state={}&code_challenge={}&code_challenge_method=S256&app_name={}",
        base_url,
        urlencoding::encode(&redirect_uri),
        urlencoding::encode(state),
        urlencoding::encode(&code_challenge),
        urlencoding::encode(app_name)
    );

    page.goto_builder(&authorize_url).goto().await?;

    // Wait for the page to load
    page.wait_for_selector_builder("#database-select")
        .timeout(5000.0)
        .wait_for_selector()
        .await?;

    // Screenshot the authorization page
    BrowserHelpers::screenshot_compare(page, "oauth_authorize_page", &[]).await?;

    // Step 2: Select the test database (created in earlier tests)
    let database_path = format!("{}/test.sqlite", username);
    page.select_option_builder("#database-select")
        .values(&[database_path.as_str()])
        .select_option()
        .await?;

    // Wait for the authorize button to become enabled
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;

    // Screenshot after database selection
    BrowserHelpers::screenshot_compare(page, "oauth_database_selected", &[]).await?;

    // Step 3: Click the authorize button
    page.click_builder("#authorize-btn")
        .timeout(5000.0)
        .click()
        .await?;

    // Step 4: Wait for redirect and capture the authorization code
    // The redirect will fail (no actual callback server), but we can capture the URL
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    let current_url = page.url()?;

    // Extract the authorization code from the URL
    let url = url::Url::parse(&current_url)?;
    let code = url
        .query_pairs()
        .find(|(k, _)| k == "code")
        .map(|(_, v)| v.to_string())
        .ok_or("No authorization code in redirect URL")?;

    let returned_state = url
        .query_pairs()
        .find(|(k, _)| k == "state")
        .map(|(_, v)| v.to_string());

    // Verify state matches
    assert_eq!(
        returned_state.as_deref(),
        Some(state),
        "State should match the original state"
    );

    println!(
        "OAuth authorization code received: {}...",
        &code[..20.min(code.len())]
    );

    // Step 5: Exchange the authorization code for an access token
    let client = reqwest::Client::new();
    let token_response = client
        .post(&format!("{}/v1/oauth/token", base_url))
        .json(&serde_json::json!({
            "grant_type": "authorization_code",
            "code": code,
            "redirect_uri": redirect_uri,
            "code_verifier": code_verifier
        }))
        .send()
        .await?;

    assert_eq!(
        token_response.status(),
        200,
        "Token exchange should succeed"
    );

    let token_data: serde_json::Value = token_response.json().await?;
    let access_token = token_data["access_token"]
        .as_str()
        .ok_or("No access_token in response")?;
    let permission_level = token_data["query_permission_level"]
        .as_str()
        .ok_or("No query_permission_level in response")?;

    assert_eq!(
        permission_level, "read-only",
        "Token should have read-only permission level"
    );
    println!("Received scoped token with {} permission", permission_level);

    // Step 6: Test that the scoped token CAN read from the database
    let read_response = client
        .post(&format!("{}/v1/{}", base_url, database_path))
        .header("Authorization", format!("Bearer {}", access_token))
        .json(&serde_json::json!({
            "query": "SELECT * FROM test_table LIMIT 1"
        }))
        .send()
        .await?;

    assert_eq!(
        read_response.status(),
        200,
        "Read-only token should be able to read from database"
    );
    println!("Successfully read from database with scoped token");

    // Step 7: Test that the scoped token CANNOT write to the database
    // This is the key test for permission capping - even though the user has
    // read-write access to their own database, the scoped token only has read-only
    let write_response = client
        .post(&format!("{}/v1/{}", base_url, database_path))
        .header("Authorization", format!("Bearer {}", access_token))
        .json(&serde_json::json!({
            "query": "INSERT INTO test_table (fname, lname) VALUES ('oauth_test', 'should_fail')"
        }))
        .send()
        .await?;

    // The write should fail because the token only has read-only scope
    assert_ne!(
        write_response.status(),
        200,
        "Read-only scoped token should NOT be able to write to database"
    );

    let error_body = write_response.text().await?;
    assert!(
        error_body.contains("read-only") || error_body.contains("ReadOnly"),
        "Error message should mention read-only restriction: {}",
        error_body
    );

    println!(
        "Confirmed: scoped read-only token cannot write to database (permission capping works)"
    );

    // Screenshot the final state (will show callback error page, which is expected)
    BrowserHelpers::screenshot_compare(page, "oauth_flow_complete", &[]).await?;

    Ok(())
}

/// Test OAuth flow with deny action
pub async fn test_oauth_deny_flow(
    page: &Page,
    username: &str,
    base_url: &str,
) -> Result<(), Box<dyn Error>> {
    let (_, code_challenge) = generate_pkce();
    let state = "deny_test_state";
    let redirect_uri = format!("{}/oauth/callback", base_url);

    let authorize_url = format!(
        "{}/oauth/authorize?response_type=code&redirect_uri={}&scope=read-only&state={}&code_challenge={}&code_challenge_method=S256&app_name={}",
        base_url,
        urlencoding::encode(&redirect_uri),
        urlencoding::encode(state),
        urlencoding::encode(&code_challenge),
        urlencoding::encode("Deny Test App")
    );

    page.goto_builder(&authorize_url).goto().await?;

    page.wait_for_selector_builder("#database-select")
        .timeout(5000.0)
        .wait_for_selector()
        .await?;

    // Select a database first (required for form submission)
    let database_path = format!("{}/test.sqlite", username);
    page.select_option_builder("#database-select")
        .values(&[database_path.as_str()])
        .select_option()
        .await?;

    // Screenshot before denying
    BrowserHelpers::screenshot_compare(page, "oauth_before_deny", &[]).await?;

    // Click the deny button
    page.click_builder("button[value='deny']")
        .timeout(5000.0)
        .click()
        .await?;

    // Wait for redirect
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    let current_url = page.url()?;

    // Verify the redirect contains an error
    let url = url::Url::parse(&current_url)?;
    let error = url
        .query_pairs()
        .find(|(k, _)| k == "error")
        .map(|(_, v)| v.to_string());

    assert_eq!(
        error.as_deref(),
        Some("access_denied"),
        "Deny action should redirect with access_denied error"
    );

    let returned_state = url
        .query_pairs()
        .find(|(k, _)| k == "state")
        .map(|(_, v)| v.to_string());

    assert_eq!(
        returned_state.as_deref(),
        Some(state),
        "State should be preserved in deny redirect"
    );

    println!("OAuth deny flow works correctly");

    Ok(())
}
