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

fn is_username_banned(username: &str) -> bool {
    static BANNED_USERNAMES: std::sync::OnceLock<HashSet<String>> = std::sync::OnceLock::new();
    let banned_set = BANNED_USERNAMES.get_or_init(load_banned_usernames);
    banned_set.contains(&username.to_lowercase())
}

/// Validate the shared syntactic rules for entity and database slugs.
///
/// Slugs are not just display names: they become path components under
/// `{data_path}/databases/...` and they are interpolated into the SQL
/// that snapshots run (`ATTACH '<path>'`, `VACUUM INTO '<path>'`). That
/// makes two character classes dangerous:
///
/// - Path separators and `.`/`..` would let a slug escape its intended
///   directory.
/// - Quotes would let a slug break out of a SQL string literal. The
///   snapshot statements escape their paths as well, but that runs in
///   the server process rather than in a sandboxed daemon, so it is
///   worth rejecting these at the boundary rather than relying on a
///   single layer.
///
/// The permitted set must stay in sync with the `pattern` attribute on
/// the create-database form in
/// `server/ui_endpoints/templates/create_database_fields.html`. That
/// attribute is a browser-side convenience only; this function is the
/// authoritative check, since the API can be called directly.
fn validate_slug_syntax(kind: &str, slug: &str) -> Result<(), AybError> {
    if slug.is_empty() {
        return Err(AybError::InvalidSlug {
            message: format!("A {kind} slug can't be empty"),
        });
    }

    if slug == "." || slug == ".." {
        return Err(AybError::InvalidSlug {
            message: format!("Invalid {kind} slug: {slug}"),
        });
    }

    if !slug
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
    {
        return Err(AybError::InvalidSlug {
            message: format!(
                "Invalid {kind} slug: {slug}. Only letters, numbers, underscores, hyphens, and periods are allowed"
            ),
        });
    }

    Ok(())
}

/// Validate an entity (user or organization) slug: the shared syntactic
/// rules, plus the reserved-name list. Entity slugs sit at the root of
/// ayb's URL space, so a name like `register` would shadow a UI route.
pub fn validate_entity_slug(slug: &str) -> Result<(), AybError> {
    validate_slug_syntax("entity", slug)?;

    if is_username_banned(slug) {
        return Err(AybError::RegistrationError {
            message: format!("Username '{slug}' is reserved and cannot be used"),
        });
    }

    Ok(())
}

/// Validate a database slug: the shared syntactic rules only. Reserved
/// names don't apply, because a database is namespaced under its entity
/// and so can't collide with a top-level route.
pub fn validate_database_slug(slug: &str) -> Result<(), AybError> {
    validate_slug_syntax("database", slug)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_accepts_ordinary_slugs() {
        for slug in ["test.sqlite", "test.duckdb", "e2e-first", "testuser_123"] {
            assert!(
                validate_database_slug(slug).is_ok(),
                "rejected database slug {slug}"
            );
            assert!(
                validate_entity_slug(slug).is_ok(),
                "rejected entity slug {slug}"
            );
        }
    }

    #[test]
    fn test_rejects_sql_and_path_metacharacters() {
        for slug in [
            "",
            ".",
            "..",
            "foo'",
            "foo';ATTACH 'x' AS y;--",
            "foo\"bar",
            "foo/bar",
            "foo\\bar",
            "../../etc/passwd",
            "foo bar",
        ] {
            assert!(
                validate_database_slug(slug).is_err(),
                "should have rejected database slug {slug:?}"
            );
            assert!(
                validate_entity_slug(slug).is_err(),
                "should have rejected entity slug {slug:?}"
            );
        }
    }

    /// Reserved names are rejected for entities but are perfectly fine
    /// for databases, which are namespaced under an entity.
    #[test]
    fn test_reserved_names_apply_only_to_entities() {
        assert!(validate_entity_slug("register").is_err());
        assert!(validate_database_slug("register").is_ok());
    }
}
