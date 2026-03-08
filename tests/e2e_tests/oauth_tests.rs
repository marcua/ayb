/// Test the OAuth token exchange endpoint error handling.
///
/// This tests error cases for the token exchange endpoint. The happy path
/// (successful token exchange) is tested in browser_e2e_tests/oauth_flow.rs,
/// which can complete the full OAuth authorization flow to obtain a valid code.
pub async fn test_oauth_token_exchange_errors(
    server_url: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("Testing OAuth token exchange endpoint (error cases)...");

    let client = reqwest::Client::new();

    // Test 1: Invalid grant_type should fail
    let response = client
        .post(&format!("{}/v1/oauth/token", server_url))
        .json(&serde_json::json!({
            "grant_type": "invalid",
            "code": "test",
            "redirect_uri": "http://localhost:3000/callback",
            "code_verifier": "test"
        }))
        .send()
        .await?;

    assert_eq!(response.status(), 400);
    let error: serde_json::Value = response.json().await?;
    assert_eq!(error["error"], "unsupported_grant_type");

    // Test 2: Invalid code should fail
    let response = client
        .post(&format!("{}/v1/oauth/token", server_url))
        .json(&serde_json::json!({
            "grant_type": "authorization_code",
            "code": "nonexistent_code",
            "redirect_uri": "http://localhost:3000/callback",
            "code_verifier": "test"
        }))
        .send()
        .await?;

    assert_eq!(response.status(), 400);
    let error: serde_json::Value = response.json().await?;
    assert_eq!(error["error"], "invalid_grant");

    println!("OAuth token exchange tests passed");
    Ok(())
}
