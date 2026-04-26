use crate::e2e_tests::{FIRST_ENTITY_DB, FIRST_ENTITY_DB2, FIRST_ENTITY_SLUG};
use crate::utils::ayb::update_database;
use serde_json::Value;
use std::collections::HashMap;

pub async fn test_anonymous_access(
    config_path: &str,
    api_keys: &HashMap<String, Vec<String>>,
    server_url: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let first_token = &api_keys.get("first").unwrap()[0];

    // Make test.sqlite publicly readable so anonymous viewers can discover it.
    update_database(
        config_path,
        first_token,
        FIRST_ENTITY_DB,
        Some("read-only"),
        "Database e2e-first/test.sqlite updated successfully",
    )?;

    let client = reqwest::Client::new();

    // GET /v1/entity/{slug} without Authorization returns 200 and only lists
    // public databases. The non-public another.sqlite is filtered out, and
    // can_create_database is false for the anonymous viewer.
    let entity_url = format!("{server_url}/v1/entity/{FIRST_ENTITY_SLUG}");
    let response = client.get(&entity_url).send().await?;
    assert_eq!(
        response.status(),
        200,
        "Anonymous entity_details should return 200"
    );
    let body: Value = response.json().await?;
    let databases = body
        .get("databases")
        .and_then(|d| d.as_array())
        .expect("databases should be an array");
    let slugs: Vec<&str> = databases
        .iter()
        .filter_map(|d| d.get("slug").and_then(|s| s.as_str()))
        .collect();
    assert_eq!(
        slugs,
        vec!["test.sqlite"],
        "Anonymous viewer should only see public databases"
    );
    assert_eq!(
        body.get("permissions")
            .and_then(|p| p.get("can_create_database"))
            .and_then(|v| v.as_bool()),
        Some(false),
        "Anonymous viewer should not be able to create databases"
    );

    // GET /v1/{entity}/{db}/details for a public read-only database returns
    // 200 with can_manage_database=false and highest_query_access_level=null.
    let details_url = format!("{server_url}/v1/{FIRST_ENTITY_DB}/details");
    let response = client.get(&details_url).send().await?;
    assert_eq!(
        response.status(),
        200,
        "Anonymous database_details on a public DB should return 200"
    );
    let body: Value = response.json().await?;
    assert_eq!(
        body.get("can_manage_database").and_then(|v| v.as_bool()),
        Some(false),
        "Anonymous viewer should not be able to manage the database"
    );
    assert!(
        body.get("highest_query_access_level")
            .map(|v| v.is_null())
            .unwrap_or(false),
        "Anonymous viewer should have no query access level"
    );
    assert_eq!(
        body.get("public_sharing_level").and_then(|v| v.as_str()),
        Some("read-only")
    );

    // GET /v1/{entity}/{db}/details for a non-public database is rejected.
    let details_url2 = format!("{server_url}/v1/{FIRST_ENTITY_DB2}/details");
    let response = client.get(&details_url2).send().await?;
    assert!(
        !response.status().is_success(),
        "Anonymous database_details on a non-public DB should be rejected"
    );

    // POST /v1/{entity}/{db}/query without Authorization is still rejected,
    // even though the database is publicly readable. Querying remains
    // authenticated.
    let query_url = format!("{server_url}/v1/{FIRST_ENTITY_DB}/query");
    let response = client.post(&query_url).body("SELECT 1").send().await?;
    assert_eq!(
        response.status(),
        401,
        "Anonymous query should be rejected with 401"
    );

    // POST /v1/{entity}/{db}/create without Authorization is rejected.
    let create_url = format!("{server_url}/v1/{FIRST_ENTITY_SLUG}/anon-test.sqlite/create");
    let response = client
        .post(&create_url)
        .header("db-type", "sqlite")
        .header("public-sharing-level", "no-access")
        .send()
        .await?;
    assert_eq!(
        response.status(),
        401,
        "Anonymous create_database should be rejected with 401"
    );

    // An invalid bearer token on the optional-auth scope is rejected, not
    // silently downgraded to anonymous.
    let response = client
        .get(&entity_url)
        .header("Authorization", "Bearer not-a-real-token")
        .send()
        .await?;
    assert!(
        !response.status().is_success(),
        "Invalid bearer token should be rejected even on optional-auth endpoints"
    );

    // Reset the database to no-access so subsequent tests start from a known
    // state.
    update_database(
        config_path,
        first_token,
        FIRST_ENTITY_DB,
        Some("no-access"),
        "Database e2e-first/test.sqlite updated successfully",
    )?;

    // After resetting to no-access, anonymous database_details is rejected.
    let response = client.get(&details_url).send().await?;
    assert!(
        !response.status().is_success(),
        "Anonymous database_details on a now-private DB should be rejected"
    );

    // Anonymous entity_details still works but contains no databases.
    let response = client.get(&entity_url).send().await?;
    assert_eq!(response.status(), 200);
    let body: Value = response.json().await?;
    let databases = body
        .get("databases")
        .and_then(|d| d.as_array())
        .expect("databases should be an array");
    assert!(
        databases.is_empty(),
        "Anonymous viewer should see no databases when none are public"
    );

    Ok(())
}
