use assert_cmd::prelude::*;
use ayb::server::snapshots::models::ListSnapshotResult;
use chrono::DateTime;
use predicates::prelude::*;
use regex::Regex;
use serde_json;
use std::process::Command;

// ayb_assert_cmd!("value1", value2; {
//     "ENV_VAR" => env_value
// })
#[macro_export]
macro_rules! ayb_assert_cmd {
    ($($value:expr),+; { $($env_left:literal => $env_right:expr),* $(,)? }) => {
        Command::cargo_bin("ayb")?
                .args([$($value,)*])
                $(.env($env_left, $env_right))*
                .assert()
                .success()
    }
}

pub fn create_database(
    config: &str,
    api_key: &str,
    database: &str,
    result: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let cmd = ayb_assert_cmd!("client", "--config", config, "create_database", database, "sqlite"; {
        "AYB_API_TOKEN" => api_key,
    });

    cmd.stdout(format!("{result}\n"));
    Ok(())
}

pub fn query(
    config: &str,
    api_key: &str,
    query: &str,
    database: &str,
    format: &str,
    result: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let cmd = ayb_assert_cmd!("client", "--config", config, "query", database, "--format", format, query; {
        "AYB_API_TOKEN" => api_key,
    });

    cmd.stdout(format!("{result}\n"));
    Ok(())
}

pub fn query_no_api_token(
    config: &str,
    query: &str,
    database: &str,
    format: &str,
    result: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let cmd = ayb_assert_cmd!("client", "--config", config, "query", database, "--format", format, query; {});

    cmd.stdout(format!("{result}\n"));
    Ok(())
}

pub fn set_default_url(
    config: &str,
    server_url: &str,
    result: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let cmd = ayb_assert_cmd!("client", "--config", config, "set_default_url", server_url; {});

    cmd.stdout(format!("{result}\n"));
    Ok(())
}

pub fn register(
    config: &str,
    server_url: &str,
    slug: &str,
    email: &str,
    result: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let cmd = ayb_assert_cmd!("client", "register", slug, email; {
        "AYB_CLIENT_CONFIG_FILE" => config,
        "AYB_SERVER_URL" => server_url,
    });

    cmd.stdout(format!("{result}\n"));
    Ok(())
}

pub fn list_databases(
    config: &str,
    api_key: &str,
    entity: &str,
    format: &str,
    result: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let cmd = ayb_assert_cmd!("client", "--config", config, "list", entity, "--format", format; {
        "AYB_API_TOKEN" => api_key,
    });

    cmd.stdout(format!("{result}\n"));
    Ok(())
}

pub fn list_snapshots(
    config: &str,
    api_key: &str,
    database: &str,
    format: &str,
) -> Result<Vec<ListSnapshotResult>, Box<dyn std::error::Error>> {
    let cmd = ayb_assert_cmd!("client", "--config", config, "list_snapshots", database, "--format", format; {
        "AYB_API_TOKEN" => api_key,
    });
    let output = std::str::from_utf8(&cmd.get_output().stdout)?;
    let mut output_lines = output.lines().collect::<Vec<&str>>();

    if output_lines.is_empty() {
        return Ok(vec![]);
    }

    assert_eq!(
        output_lines[0], "Name,Last modified",
        "first result line should be a header row"
    );
    let re = Regex::new(r"([a-f0-9]{64}),(\d{4,5}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(?:\.\d+)?\+00:00)")
        .unwrap();
    let mut snapshots = Vec::new();
    for line in &mut output_lines[1..] {
        let capture = re
            .captures(line)
            .expect("resulting line should be a snapshot record");
        snapshots.push(ListSnapshotResult {
            snapshot_id: capture
                .get(1)
                .expect("snapshot line should have a hash/id")
                .as_str()
                .to_string(),
            last_modified_at: DateTime::parse_from_rfc3339(
                capture
                    .get(2)
                    .expect("snapshot line should have a datetime")
                    .into(),
            )
            .expect("datetime should be in ISO format")
            .into(),
        })
    }

    Ok(snapshots)
}

pub fn list_snapshots_match_output(
    config: &str,
    api_key: &str,
    database: &str,
    format: &str,
    result: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let cmd = ayb_assert_cmd!("client", "--config", config, "list_snapshots", database, "--format", format; {
        "AYB_API_TOKEN" => api_key,
    });

    cmd.stdout(predicate::str::is_match(format!("{result}\n")).unwrap());
    Ok(())
}

pub fn restore_snapshot(
    config: &str,
    api_key: &str,
    database: &str,
    snapshot_id: &str,
    result: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let cmd = ayb_assert_cmd!("client", "--config", config, "restore_snapshot", database, snapshot_id; {
        "AYB_API_TOKEN" => api_key,
    });

    cmd.stdout(format!("{result}\n"));
    Ok(())
}

pub fn profile(
    config: &str,
    api_key: &str,
    entity: &str,
    format: &str,
    result: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let cmd = ayb_assert_cmd!("client", "--config", config, "profile", entity, "--format", format; {
        "AYB_API_TOKEN" => api_key,
    });

    cmd.stdout(format!("{result}\n"));
    Ok(())
}

pub fn update_profile(
    config: &str,
    api_key: &str,
    entity: &str,
    display_name: Option<&str>,
    description: Option<&str>,
    organization: Option<&str>,
    location: Option<&str>,
    links: Option<Vec<&str>>,
    result: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = Command::cargo_bin("ayb")?;
    cmd.args(["client", "--config", config, "update_profile", entity])
        .env("AYB_API_TOKEN", api_key);

    if let Some(display_name) = display_name {
        cmd.arg("--display_name").arg(display_name);
    }

    if let Some(description) = description {
        cmd.arg("--description").arg(description);
    }

    if let Some(organization) = organization {
        cmd.arg("--organization").arg(organization);
    }

    if let Some(location) = location {
        cmd.arg("--location").arg(location);
    }

    if let Some(links) = links {
        cmd.arg("--links").arg(links.join(","));
    }

    cmd.assert().success().stdout(format!("{result}\n"));
    Ok(())
}

pub fn update_database(
    config: &str,
    api_key: &str,
    database: &str,
    public_sharing_level: Option<&str>,
    result: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = Command::cargo_bin("ayb")?;
    cmd.args(["client", "--config", config, "update_database", database])
        .env("AYB_API_TOKEN", api_key);

    if let Some(level) = public_sharing_level {
        cmd.arg("--public_sharing_level").arg(level);
        cmd.assert().success().stdout(format!("{result}\n"));
    } else {
        cmd.assert()
            .failure()
            .stderr(predicates::str::contains(result));
    }
    Ok(())
}

pub fn share(
    config: &str,
    api_key: &str,
    database: &str,
    entity: &str,
    sharing_level: &str,
    result: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let cmd = ayb_assert_cmd!("client", "--config", config, "share", database, entity, sharing_level; {
        "AYB_API_TOKEN" => api_key,
    });

    cmd.stdout(format!("{result}\n"));
    Ok(())
}

pub fn database_details(
    config: &str,
    api_key: &str,
    database: &str,
    result: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let cmd = ayb_assert_cmd!("client", "--config", config, "database_details", database; {
        "AYB_API_TOKEN" => api_key,
    });

    cmd.stdout(predicate::str::contains(result));
    Ok(())
}

pub fn list_database_permissions(
    config: &str,
    api_key: &str,
    database: &str,
    format: &str,
    result: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let cmd = ayb_assert_cmd!("client", "--config", config, "list_database_permissions", database, "--format", format; {
        "AYB_API_TOKEN" => api_key,
    });

    cmd.stdout(format!("{result}\n"));
    Ok(())
}

pub fn list_tokens(
    config: &str,
    api_key: &str,
    format: &str,
    result: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let cmd = ayb_assert_cmd!("client", "--config", config, "list_tokens", "--format", format; {
        "AYB_API_TOKEN" => api_key,
    });

    cmd.stdout(predicate::str::contains(result));
    Ok(())
}

/// List tokens and return the short tokens as a Vec for assertions
pub fn list_tokens_json(
    config: &str,
    api_key: &str,
) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let output = Command::cargo_bin("ayb")?
        .args([
            "client",
            "--config",
            config,
            "list_tokens",
            "--format",
            "json",
        ])
        .env("AYB_API_TOKEN", api_key)
        .output()?;

    let stdout = String::from_utf8(output.stdout)?;
    // Parse JSON response to extract short tokens
    // The response is an array of token objects with "short_token" field
    let tokens: Vec<serde_json::Value> = serde_json::from_str(&stdout)?;
    let short_tokens: Vec<String> = tokens
        .iter()
        .filter_map(|t| {
            t.get("short_token")
                .and_then(|v| v.as_str())
                .map(String::from)
        })
        .collect();
    Ok(short_tokens)
}

pub fn revoke_token(
    config: &str,
    api_key: &str,
    short_token: &str,
    result: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let cmd = ayb_assert_cmd!("client", "--config", config, "revoke_token", short_token; {
        "AYB_API_TOKEN" => api_key,
    });

    cmd.stdout(format!("{result}\n"));
    Ok(())
}
