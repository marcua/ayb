use crate::utils::testing::get_test_port;

pub async fn test_health_check(test_type: &str) -> Result<(), Box<dyn std::error::Error>> {
    let port = get_test_port(test_type)?;
    let url = format!("http://127.0.0.1:{}/health", port);

    // Make a request to the health endpoint
    let response = reqwest::get(&url).await?;

    // Verify the status code is 200
    assert_eq!(
        response.status(),
        200,
        "Health endpoint should return 200 OK"
    );

    // Verify the response body
    let body: serde_json::Value = response.json().await?;
    assert_eq!(
        body.get("status").and_then(|v| v.as_str()),
        Some("ok"),
        "Health endpoint should return {{\"status\": \"ok\"}}"
    );

    Ok(())
}
