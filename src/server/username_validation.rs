use crate::error::AybError;
use reqwest;
use std::collections::HashSet;

/// Reserved usernames loaded from file with comprehensive list
///
/// Original list source: https://github.com/shouldbee/reserved-usernames
/// Licensed under MIT License (see reserved-usernames.txt for full attribution)
///
/// Additional ayb-specific reserved names have been added to prevent conflicts
/// with ayb's UI routes and system paths.
const RESERVED_USERNAMES_RAW: &str = include_str!("reserved-usernames.txt");

fn load_banned_usernames() -> HashSet<String> {
    RESERVED_USERNAMES_RAW
        .lines()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(|line| line.to_lowercase())
        .collect()
}

pub fn is_username_banned(username: &str) -> bool {
    static BANNED_USERNAMES: std::sync::OnceLock<HashSet<String>> = std::sync::OnceLock::new();
    let banned_set = BANNED_USERNAMES.get_or_init(load_banned_usernames);
    banned_set.contains(&username.to_lowercase())
}

pub async fn validate_username_route_conflict(
    username: &str,
    base_url: &str,
) -> Result<(), AybError> {
    let client = reqwest::Client::new();
    let url = format!("{}/{}", base_url.trim_end_matches('/'), username);

    match client.head(&url).send().await {
        Ok(response) => {
            if response.status().is_success() {
                return Err(AybError::Other {
                    message: format!("Username '{}' conflicts with existing route", username),
                });
            }
        }
        Err(_) => {
            // If we can't make the request, we'll skip this validation
            // and rely on the banned username list
        }
    }

    Ok(())
}

pub async fn validate_username(username: &str, base_url: Option<&str>) -> Result<(), AybError> {
    // Check banned username list first (fast check)
    if is_username_banned(username) {
        return Err(AybError::Other {
            message: format!("Username '{}' is reserved and cannot be used", username),
        });
    }

    // If base_url is provided, also check for route conflicts
    if let Some(url) = base_url {
        validate_username_route_conflict(username, url).await?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_banned_usernames() {
        // Test original reserved usernames from shouldbee/reserved-usernames
        let banned = [
            "admin", "root", "www", "api", "support", "help", "blog", "news",
        ];
        for username in banned {
            assert!(
                is_username_banned(username),
                "Username '{}' should be banned",
                username
            );
        }

        // Test ayb-specific reserved usernames
        let ayb_banned = ["register", "log_in", "log_out", "confirm", "v1"];
        for username in ayb_banned {
            assert!(
                is_username_banned(username),
                "Username '{}' should be banned (ayb-specific)",
                username
            );
        }
    }

    #[test]
    fn test_valid_usernames() {
        let valid = ["alice", "bob123", "my-company", "user_2024", "testuser"];
        for username in valid {
            assert!(
                !is_username_banned(username),
                "Username '{}' should be allowed",
                username
            );
        }
    }

    #[test]
    fn test_case_insensitive_banned() {
        assert!(is_username_banned("REGISTER"));
        assert!(is_username_banned("Register"));
        assert!(is_username_banned("Log_In"));
        assert!(is_username_banned("API"));
    }

    #[tokio::test]
    async fn test_validate_username_banned() {
        let result = validate_username("register", None).await;
        assert!(result.is_err());

        let result = validate_username("validusername", None).await;
        assert!(result.is_ok());
    }
}
