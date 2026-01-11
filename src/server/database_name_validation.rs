use crate::error::AybError;

/// Reserved database names that cannot be used to prevent route conflicts.
/// Currently empty - entity names like "settings" are reserved via username_validation.rs.
const RESERVED_DATABASE_NAMES: &[&str] = &[];

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
