use crate::e2e_tests::{FIRST_ENTITY_DB, FIRST_ENTITY_DB2, FIRST_ENTITY_SLUG};
use crate::utils::ayb::{
    database_details_no_auth, list_databases, list_databases_no_auth, update_database,
};
use std::collections::HashMap;

pub async fn test_anonymous_access(
    config_path: &str,
    api_keys: &HashMap<String, Vec<String>>,
    server_url: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let first_token = &api_keys.get("first").unwrap()[0];

    // CLI config used for anonymous calls — pointed at a tempfile so it
    // can't pick up cached tokens for `server_url` from the main test
    // config. The CLI may write `default_url` into this file the first
    // time `--url` is passed, but never any auth.
    let anon_config_dir = tempfile::tempdir()?;
    let anon_config = anon_config_dir.path().join("anon.json");
    let anon_config = anon_config.to_str().unwrap();

    // Make test.sqlite publicly readable so anonymous viewers can discover it.
    update_database(
        config_path,
        first_token,
        FIRST_ENTITY_DB,
        Some("read-only"),
        "Database e2e-first/test.sqlite updated successfully",
    )?;

    // `ayb client list` without an API token returns only public databases —
    // the non-public another.sqlite is filtered out.
    list_databases_no_auth(
        anon_config,
        server_url,
        FIRST_ENTITY_SLUG,
        "csv",
        "Database slug,Type\ntest.sqlite,sqlite",
    )?;

    // `ayb client database_details` without an API token succeeds for a
    // publicly read-only database, with no query access and no management.
    database_details_no_auth(
        anon_config,
        server_url,
        FIRST_ENTITY_DB,
        "Database: e2e-first/test.sqlite\nType: sqlite\nAccess level: No query access",
    )?;

    // The same call against a non-public database is rejected with the
    // anon-friendly message (no leak of the requesting entity's identity,
    // because there is none).
    database_details_no_auth(
        anon_config,
        server_url,
        FIRST_ENTITY_DB2,
        "Error: Database e2e-first/another.sqlite is not accessible",
    )?;

    // An invalid bearer token on the optional-auth scope is rejected — not
    // silently downgraded to anonymous. We exercise this through the CLI
    // by appending garbage to a real token (preserving the prefix/parts
    // shape so the server actually validates it rather than rejecting at
    // the parser stage).
    list_databases(
        config_path,
        &format!("{first_token}bad"),
        FIRST_ENTITY_SLUG,
        "csv",
        "Error: Invalid API token",
    )?;

    // Querying and creating remain authenticated. The CLI's own
    // `add_bearer_token(optional=false)` fails before reaching the server
    // when no token is set, so we verify the *server-side* rejection
    // directly with reqwest. Disable connection pooling so the empty-body
    // 401 response from `HttpAuthentication::bearer` doesn't leave a
    // stale connection that the next request would race on.
    let client = reqwest::Client::builder()
        .pool_max_idle_per_host(0)
        .build()?;

    let query_url = format!("{server_url}/v1/{FIRST_ENTITY_DB}/query");
    let response = client.post(&query_url).body("SELECT 1").send().await?;
    assert_eq!(
        response.status(),
        401,
        "Anonymous query should be rejected with 401"
    );

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

    // Reset the database to no-access so subsequent tests start from a
    // known state.
    update_database(
        config_path,
        first_token,
        FIRST_ENTITY_DB,
        Some("no-access"),
        "Database e2e-first/test.sqlite updated successfully",
    )?;

    // After resetting, anonymous database_details on the now-private DB is
    // rejected.
    database_details_no_auth(
        anon_config,
        server_url,
        FIRST_ENTITY_DB,
        "Error: Database e2e-first/test.sqlite is not accessible",
    )?;

    // Anonymous list still succeeds, but contains no databases.
    list_databases_no_auth(
        anon_config,
        server_url,
        FIRST_ENTITY_SLUG,
        "csv",
        &format!("No queryable databases owned by {FIRST_ENTITY_SLUG}"),
    )?;

    Ok(())
}
