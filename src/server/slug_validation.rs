use crate::error::AybError;

/// Validate an entity or database slug.
///
/// Slugs are not just display names: they become path components under
/// `{data_path}/databases/...` and they are interpolated into the SQL
/// that snapshots run (`ATTACH '<path>'`, `VACUUM INTO '<path>'`). That
/// makes two characters classes dangerous:
///
/// - Path separators and `.`/`..` would let a slug escape its intended
///   directory.
/// - Quotes would let a slug break out of a SQL string literal. The
///   snapshot statements escape their paths as well, but that runs in
///   the server process rather than in a sandboxed daemon, so it is
///   worth rejecting these at the boundary rather than relying on a
///   single layer.
///
/// The permitted set matches the `pattern` attribute on the
/// create-database form, which previously existed only in the browser
/// and so was trivially bypassed by calling the API directly.
pub fn validate_slug(kind: &str, slug: &str) -> Result<(), AybError> {
    if slug.is_empty() {
        return Err(AybError::Other {
            message: format!("A {kind} slug can't be empty"),
        });
    }

    if slug == "." || slug == ".." {
        return Err(AybError::Other {
            message: format!("Invalid {kind} slug: {slug}"),
        });
    }

    if !slug
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
    {
        return Err(AybError::Other {
            message: format!(
                "Invalid {kind} slug: {slug}. Only letters, numbers, underscores, hyphens, and periods are allowed"
            ),
        });
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_accepts_ordinary_slugs() {
        for slug in ["test.sqlite", "test.duckdb", "e2e-first", "testuser_123"] {
            assert!(validate_slug("database", slug).is_ok(), "rejected {slug}");
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
                validate_slug("database", slug).is_err(),
                "should have rejected {slug:?}"
            );
        }
    }
}
