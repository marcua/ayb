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

/// Remove Unicode bidirectional-control and invisible-formatting
/// characters from user-supplied profile text.
///
/// Unlike slugs, profile fields (display name, description,
/// organization, location, and link URLs) are where a user is expected
/// to write their real name in whatever script it belongs to, so they
/// accept arbitrary Unicode. Tera auto-escapes them, which handles HTML
/// injection, but escaping leaves these characters intact, and they act
/// on the text *around* them once rendered:
///
/// - Bidirectional controls (`U+061C`, `U+200E`, `U+200F`,
///   `U+202A`..=`U+202E`, `U+2066`..=`U+2069`) reorder neighboring text.
///   A display name containing RIGHT-TO-LEFT OVERRIDE (`U+202E`)
///   rewrites how the slug, database names, and sharing controls shown
///   next to it appear. This is the "Trojan Source" class of attack
///   (CVE-2021-42574), and it matters here because slugs are ASCII-only,
///   which makes the display name the only Unicode-accepting field a
///   viewer actually reads.
/// - Zero-width and other invisible characters (`U+00AD`, `U+200B`,
///   `U+200C`, `U+200D`, `U+2060`, `U+FEFF`) let two different profiles
///   render identically, which aids impersonation in listings.
///
/// We strip rather than reject: these characters are not something a
/// user types on purpose in a name, so removing them silently is
/// friendlier than an error nobody can act on. Stripping happens where
/// the profile is written rather than where it's read, so the stored
/// value is clean for every consumer — the web UI, CLI output, the JSON
/// API, and email.
///
/// Every other code point is preserved, so names in Arabic, Hebrew, CJK,
/// Cyrillic, accented Latin, and so on round-trip untouched. The one
/// tradeoff is that `U+200C` (ZWNJ) and `U+200D` (ZWJ) are meaningful in
/// Persian and in Indic scripts, and join emoji sequences; a name that
/// legitimately uses them will render slightly differently once they're
/// removed.
pub fn strip_bidi_and_invisible(text: &str) -> String {
    text.chars()
        .filter(|c| {
            !matches!(*c,
                '\u{00AD}'                 // SOFT HYPHEN
                | '\u{061C}'               // ARABIC LETTER MARK
                | '\u{200B}'..='\u{200F}'  // zero-width characters, LRM, RLM
                | '\u{202A}'..='\u{202E}'  // LRE, RLE, PDF, LRO, RLO
                | '\u{2060}'               // WORD JOINER
                | '\u{2066}'..='\u{2069}'  // LRI, RLI, FSI, PDI
                | '\u{FEFF}'               // ZERO WIDTH NO-BREAK SPACE
            )
        })
        .collect()
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

    /// The characters that let a profile field rewrite the text around
    /// it, or render identically to a different profile, are removed.
    #[test]
    fn test_strips_bidi_and_invisible_characters() {
        for (input, expected) in [
            // Trojan Source: an override that reorders what follows it.
            ("Entity\u{202E} 0", "Entity 0"),
            ("\u{202E}gnp.eliforp", "gnp.eliforp"),
            // The rest of the bidi controls, embeddings and isolates.
            ("a\u{202A}b\u{202B}c\u{202C}d\u{202D}e", "abcde"),
            ("a\u{2066}b\u{2067}c\u{2068}d\u{2069}e", "abcde"),
            ("a\u{200E}b\u{200F}c\u{061C}d", "abcd"),
            // Invisible characters that make two names look the same.
            ("Ent\u{200B}ity 0", "Entity 0"),
            ("Ent\u{00AD}ity 0", "Entity 0"),
            ("Ent\u{FEFF}ity 0", "Entity 0"),
            ("Ent\u{2060}ity 0", "Entity 0"),
            ("Ent\u{200C}it\u{200D}y 0", "Entity 0"),
            // Text that needs no cleaning is returned unchanged.
            ("", ""),
            ("Entity 0", "Entity 0"),
        ] {
            assert_eq!(
                strip_bidi_and_invisible(input),
                expected,
                "unexpected result for {input:?}"
            );
        }
    }

    /// The point of stripping only formatting controls is that every
    /// real script still round-trips: profile fields are where users
    /// write their actual name.
    #[test]
    fn test_preserves_ordinary_unicode_names() {
        for name in [
            "Entity 0",
            "أحمد بن سليمان",     // Arabic
            "משה בן־מימון",       // Hebrew
            "北京大学",           // Chinese
            "やまだ たろう",      // Japanese
            "Ясен Стойчев",       // Cyrillic
            "José Ángel Peña",    // Accented Latin
            "Δημήτριος",          // Greek
            "ᏣᎳᎩ",                // Cherokee
            "Data — everywhere!", // Punctuation and symbols
            "🐘 postgres fan",    // Emoji
        ] {
            assert_eq!(strip_bidi_and_invisible(name), name, "modified {name:?}");
        }
    }
}
