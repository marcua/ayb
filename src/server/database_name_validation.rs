use crate::error::AybError;

/// Reserved database names that cannot be used to prevent route conflicts.
/// The `-` is reserved because it's used as a system route prefix (e.g., /{entity}/-/tokens).
const RESERVED_DATABASE_NAMES: &[&str] = &["-"];

pub fn is_database_name_reserved(name: &str) -> bool {
    RESERVED_DATABASE_NAMES.contains(&name.to_lowercase().as_str())
}

pub fn validate_database_name(name: &str) -> Result<(), AybError> {
    if is_database_name_reserved(name) {
        return Err(AybError::Other {
            message: format!("Database name '{name}' is reserved and cannot be used"),
        });
    }
    Ok(())
}
