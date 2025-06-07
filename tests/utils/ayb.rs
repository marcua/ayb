use assert_cmd::prelude::*;
use ayb::server::snapshots::models::ListSnapshotResult;
use chrono::DateTime;
use predicates::prelude::*;
use regex::Regex;
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

    cmd.stdout(format!("{}\n", result));
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

    cmd.stdout(format!("{}\n", result));
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

    cmd.stdout(format!("{}\n", result));
    Ok(())
}

pub fn set_default_url(
    config: &str,
    server_url: &str,
    result: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let cmd = ayb_assert_cmd!("client", "--config", config, "set_default_url", server_url; {});

    cmd.stdout(format!("{}\n", result));
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

    cmd.stdout(format!("{}\n", result));
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

    cmd.stdout(format!("{}\n", result));
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
    let mut output_lines = std::str::from_utf8(&cmd.get_output().stdout)?
        .lines()
        .collect::<Vec<&str>>();
    assert_eq!(
        output_lines[0], "Name,Last modified",
        "first result line should be a header row"
    );
    let re = Regex::new(r"([a-f0-9]{64}),(\d{4,5}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}\+00:00)").unwrap();
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

    cmd.stdout(predicate::str::is_match(format!("{}\n", result)).unwrap());
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

    cmd.stdout(format!("{}\n", result));
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

    cmd.stdout(format!("{}\n", result));
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

    cmd.assert().success().stdout(format!("{}\n", result));
    Ok(())
}

pub fn update_database(
    config: &str,
    api_key: &str,
    database: &str,
    public_sharing_level: &str,
    result: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let cmd = ayb_assert_cmd!("client", "--config", config, "update_database", database, "--public_sharing_level", public_sharing_level; {
        "AYB_API_TOKEN" => api_key,
    });

    cmd.stdout(format!("{}\n", result));
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

    cmd.stdout(format!("{}\n", result));
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

pub fn share_list(
    config: &str,
    api_key: &str,
    database: &str,
    format: &str,
    result: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let cmd = ayb_assert_cmd!("client", "--config", config, "share_list", database, "--format", format; {
        "AYB_API_TOKEN" => api_key,
    });

    cmd.stdout(format!("{}\n", result));
    Ok(())
}
