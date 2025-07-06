use crate::error::AybError;
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

pub fn validate_username(username: &str) -> Result<(), AybError> {
    // Check banned username list
    if is_username_banned(username) {
        return Err(AybError::Other {
            message: format!("Username '{}' is reserved and cannot be used", username),
        });
    }

    Ok(())
}
