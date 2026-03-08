use crate::utils::browser::BrowserHelpers;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use playwright::api::Page;
use sha2::{Digest, Sha256};
use std::error::Error;

/// Generate a PKCE code verifier and challenge pair.
/// Uses the same SHA256 + base64url approach as the server's verify_pkce function.
fn generate_pkce() -> (String, String) {
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

    let mut hasher = Sha256::new();
    hasher.update(verifier.as_bytes());
    let hash = hasher.finalize();
    let challenge = URL_SAFE_NO_PAD.encode(hash);

    (verifier, challenge)
}

/// Result of completing the OAuth authorization flow
struct OAuthTokenResult {
    access_token: String,
    permission_level: String,
}

/// Complete the OAuth authorization flow and exchange for a token.
///
/// This handles the browser-based authorization flow:
/// 1. Navigate to the OAuth authorize page
/// 2. Select a database
/// 3. Click authorize
/// 4. Capture the authorization code from the redirect
/// 5. Exchange the code for an access token
async fn complete_oauth_flow(
    page: &Page,
    username: &str,
    base_url: &str,
    scope: &str,
    screenshot_prefix: &str,
) -> Result<OAuthTokenResult, Box<dyn Error>> {
    let (code_verifier, code_challenge) = generate_pkce();
    let state = format!("test_state_{}", scope.replace('-', "_"));
    let app_name = format!("Test OAuth App ({})", scope);

    // This is a fake URL. We use it to capture the redirect and
    // extract the authorization code from the URL query
    // parameters after the browser redirects.
    let redirect_uri = format!("{}/oauth/callback", base_url);

    let authorize_url = format!(
        "{}/oauth/authorize?response_type=code&redirect_uri={}&scope={}&state={}&code_challenge={}&code_challenge_method=S256&app_name={}",
        base_url,
        urlencoding::encode(&redirect_uri),
        urlencoding::encode(scope),
        urlencoding::encode(&state),
        urlencoding::encode(&code_challenge),
        urlencoding::encode(&app_name)
    );

    page.goto_builder(&authorize_url).goto().await?;

    page.wait_for_selector_builder("#database-select")
        .timeout(5000.0)
        .wait_for_selector()
        .await?;

    BrowserHelpers::screenshot_compare(
        page,
        &format!("oauth_{}_authorize_page", screenshot_prefix),
        &[],
    )
    .await?;

    let database_path = format!("{}/test.sqlite", username);
    page.select_option_builder("#database-select")
        .values(&[database_path.as_str()])
        .select_option()
        .await?;

    tokio::time::sleep(std::time::Duration::from_millis(200)).await;

    BrowserHelpers::screenshot_compare(
        page,
        &format!("oauth_{}_database_selected", screenshot_prefix),
        &[],
    )
    .await?;

    page.click_builder("#authorize-btn")
        .timeout(5000.0)
        .click()
        .await?;

    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    let current_url = page.url()?;
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

    assert_eq!(
        returned_state.as_deref(),
        Some(state.as_str()),
        "State should match the original state"
    );

    println!(
        "OAuth authorization code received for {} scope: {}...",
        scope,
        &code[..20.min(code.len())]
    );

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
        .ok_or("No access_token in response")?
        .to_string();
    let permission_level = token_data["query_permission_level"]
        .as_str()
        .ok_or("No query_permission_level in response")?
        .to_string();

    assert_eq!(
        permission_level, scope,
        "Token should have {} permission level",
        scope
    );

    println!("Received scoped token with {} permission", permission_level);

    Ok(OAuthTokenResult {
        access_token,
        permission_level,
    })
}

/// Test the OAuth flow with a read-only scoped token.
///
/// This verifies that:
/// 1. A user can authorize an OAuth request for read-only access
/// 2. The returned token can be used to read from the database
/// 3. The returned token cannot be used to write to the database (permission capping)
pub async fn test_oauth_flow_readonly(
    page: &Page,
    username: &str,
    base_url: &str,
) -> Result<String, Box<dyn Error>> {
    println!("Testing OAuth flow with read-only scope...");

    let result = complete_oauth_flow(page, username, base_url, "read-only", "readonly").await?;
    let database_path = format!("{}/test.sqlite", username);
    let client = reqwest::Client::new();

    // Test that the scoped token can read from the database
    let read_response = client
        .post(&format!("{}/v1/{}", base_url, database_path))
        .header("Authorization", format!("Bearer {}", result.access_token))
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
    println!("Successfully read from database with read-only scoped token");

    // Test that the scoped token cannot write to the database.
    // This is the key test for permission capping - even though the user has
    // read-write access to their own database, the scoped token only has read-only.
    let write_response = client
        .post(&format!("{}/v1/{}", base_url, database_path))
        .header("Authorization", format!("Bearer {}", result.access_token))
        .json(&serde_json::json!({
            "query": "INSERT INTO test_table (fname, lname) VALUES ('oauth_readonly_test', 'should_fail')"
        }))
        .send()
        .await?;

    assert_ne!(
        write_response.status(),
        200,
        "Read-only scoped token should not be able to write to database"
    );

    let error_body = write_response.text().await?;
    assert!(
        error_body.contains("Attempted to write to database while in read-only mode"),
        "Error message should be 'Attempted to write to database while in read-only mode', got: {}",
        error_body
    );

    // Verify the row was not inserted
    assert!(
        !error_body.contains("oauth_readonly_test"),
        "Response should not contain the test data"
    );

    println!(
        "Confirmed: scoped read-only token cannot write to database (permission capping works)"
    );

    BrowserHelpers::screenshot_compare(page, "oauth_readonly_flow_complete", &[]).await?;

    Ok(result.access_token)
}

/// Test the OAuth flow with a read-write scoped token.
///
/// This verifies that:
/// 1. A user can authorize an OAuth request for read-write access
/// 2. The returned token can be used to read from the database
/// 3. The returned token can be used to write to the database (no capping for read-write)
pub async fn test_oauth_flow_readwrite(
    page: &Page,
    username: &str,
    base_url: &str,
) -> Result<String, Box<dyn Error>> {
    println!("Testing OAuth flow with read-write scope...");

    let result = complete_oauth_flow(page, username, base_url, "read-write", "readwrite").await?;
    let database_path = format!("{}/test.sqlite", username);
    let client = reqwest::Client::new();

    // Test that the scoped token can read from the database
    let read_response = client
        .post(&format!("{}/v1/{}", base_url, database_path))
        .header("Authorization", format!("Bearer {}", result.access_token))
        .json(&serde_json::json!({
            "query": "SELECT * FROM test_table LIMIT 1"
        }))
        .send()
        .await?;

    assert_eq!(
        read_response.status(),
        200,
        "Read-write token should be able to read from database"
    );
    println!("Successfully read from database with read-write scoped token");

    // Test that the scoped token can write to the database.
    // This verifies that read-write scope is not capped.
    let write_response = client
        .post(&format!("{}/v1/{}", base_url, database_path))
        .header("Authorization", format!("Bearer {}", result.access_token))
        .json(&serde_json::json!({
            "query": "INSERT INTO test_table (fname, lname) VALUES ('oauth_readwrite_test', 'should_succeed')"
        }))
        .send()
        .await?;

    assert_eq!(
        write_response.status(),
        200,
        "Read-write scoped token should be able to write to database"
    );

    println!("Confirmed: scoped read-write token can write to database (not capped)");

    // Verify the write actually worked
    let verify_response = client
        .post(&format!("{}/v1/{}", base_url, database_path))
        .header("Authorization", format!("Bearer {}", result.access_token))
        .json(&serde_json::json!({
            "query": "SELECT * FROM test_table WHERE fname = 'oauth_readwrite_test'"
        }))
        .send()
        .await?;

    assert_eq!(verify_response.status(), 200);
    let verify_body = verify_response.text().await?;
    assert!(
        verify_body.contains("oauth_readwrite_test"),
        "Should find the inserted row: {}",
        verify_body
    );

    println!("Verified: row was successfully inserted with read-write scoped token");

    BrowserHelpers::screenshot_compare(page, "oauth_readwrite_flow_complete", &[]).await?;

    Ok(result.access_token)
}

/// Test the complete OAuth flow including both read-only and read-write scoped tokens.
///
/// This is the main entry point that tests:
/// 1. Read-only tokens are properly capped (cannot write)
/// 2. Read-write tokens are not capped (can write)
pub async fn test_oauth_flow(
    page: &Page,
    username: &str,
    base_url: &str,
) -> Result<(String, String), Box<dyn Error>> {
    // Test read-only scope (should be capped)
    let readonly_token = test_oauth_flow_readonly(page, username, base_url).await?;

    // Test read-write scope (should not be capped)
    let readwrite_token = test_oauth_flow_readwrite(page, username, base_url).await?;

    println!("All OAuth flow tests passed successfully");

    Ok((readonly_token, readwrite_token))
}

/// Test OAuth flow with deny action
pub async fn test_oauth_deny_flow(
    page: &Page,
    username: &str,
    base_url: &str,
) -> Result<(), Box<dyn Error>> {
    let (_, code_challenge) = generate_pkce();
    let state = "deny_test_state";

    // This is a fake URL. We use it to capture the redirect and
    // extract the error from the URL query parameters.
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

    let database_path = format!("{}/test.sqlite", username);
    page.select_option_builder("#database-select")
        .values(&[database_path.as_str()])
        .select_option()
        .await?;

    BrowserHelpers::screenshot_compare(page, "oauth_before_deny", &[]).await?;

    page.click_builder("button[value='deny']")
        .timeout(5000.0)
        .click()
        .await?;

    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    let current_url = page.url()?;

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
