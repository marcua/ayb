use regex::Regex;
use std::sync::OnceLock;
use url::Url;

// Pre-filter for candidate <a> tags: requires a `rel` attribute whose value
// contains a `me` token (in any quote style, or unquoted). The strict
// per-tag check still validates href and exact rel-token semantics.
fn anchor_regex() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| {
        Regex::new(
            r#"(?is)<a\b([^>]*\brel\s*=\s*(?:"[^"]*\bme\b[^"]*"|'[^']*\bme\b[^']*'|me\b)[^>]*)>"#,
        )
        .unwrap()
    })
}

fn attr_regex() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| {
        Regex::new(r#"(?is)([a-zA-Z_:][-a-zA-Z0-9_:.]*)\s*=\s*(?:"([^"]*)"|'([^']*)'|([^\s>]+))"#)
            .unwrap()
    })
}

/// Returns true if `html` contains an `<a>` element whose `href` equals
/// `expected_url` and whose `rel` attribute contains the token `me`. Per the
/// HTML spec `rel` is a space-separated token list, so values like
/// `rel="me author"` are valid matches.
pub fn html_has_rel_me_link(html: &str, expected_url: &str) -> bool {
    // Cheap short-circuit: if the URL is nowhere on the page, no anchor can match.
    if !html.contains(expected_url) {
        return false;
    }
    for tag in anchor_regex().captures_iter(html) {
        let attrs_blob = &tag[1];
        let mut href: Option<&str> = None;
        let mut rel: Option<&str> = None;
        for cap in attr_regex().captures_iter(attrs_blob) {
            let name = cap.get(1).unwrap().as_str();
            let value = cap.get(2).or(cap.get(3)).or(cap.get(4)).unwrap().as_str();
            if name.eq_ignore_ascii_case("href") {
                href = Some(value);
            } else if name.eq_ignore_ascii_case("rel") {
                rel = Some(value);
            }
        }
        if href == Some(expected_url)
            && rel.is_some_and(|r| {
                r.split_ascii_whitespace()
                    .any(|tok| tok.eq_ignore_ascii_case("me"))
            })
        {
            return true;
        }
    }
    false
}

/// Verifies that the HTML website located at `input_url` contains an anchor tag whose `rel` attribute is set to `me`
/// and whose `href` attribute is equal to the value of `expected_url`.
pub async fn is_verified_url(input_url: Url, expected_url: Url) -> bool {
    if input_url.scheme() != "https" {
        return false;
    }

    if let Ok(website) = reqwest::get(input_url.to_string()).await {
        if let Ok(body) = website.text().await {
            return html_has_rel_me_link(&body, expected_url.as_ref());
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;

    const URL: &str = "https://ayb.host/u/me";

    #[test]
    fn positive_basic() {
        assert!(html_has_rel_me_link(
            r#"<a href="https://ayb.host/u/me" rel="me">x</a>"#,
            URL
        ));
    }

    #[test]
    fn positive_attrs_reversed() {
        assert!(html_has_rel_me_link(
            r#"<a rel="me" href="https://ayb.host/u/me">x</a>"#,
            URL
        ));
    }

    #[test]
    fn positive_multi_token_rel() {
        assert!(html_has_rel_me_link(
            r#"<a href="https://ayb.host/u/me" rel="me author">x</a>"#,
            URL
        ));
    }

    #[test]
    fn positive_single_quotes() {
        assert!(html_has_rel_me_link(
            r#"<a href='https://ayb.host/u/me' rel='me'>x</a>"#,
            URL
        ));
    }

    #[test]
    fn positive_intervening_attrs() {
        assert!(html_has_rel_me_link(
            r#"<a class="foo" href="https://ayb.host/u/me" data-x="y" rel="me">x</a>"#,
            URL
        ));
    }

    #[test]
    fn positive_uppercase_attrs() {
        assert!(html_has_rel_me_link(
            r#"<A HREF="https://ayb.host/u/me" REL="ME">x</A>"#,
            URL
        ));
    }

    #[test]
    fn negative_wrong_href() {
        assert!(!html_has_rel_me_link(
            r#"<a href="https://other.example/u" rel="me">x</a>"#,
            URL
        ));
    }

    #[test]
    fn negative_rel_me_no_href() {
        assert!(!html_has_rel_me_link(r#"<a rel="me">no href</a>"#, URL));
    }

    #[test]
    fn negative_different_rel() {
        assert!(!html_has_rel_me_link(
            r#"<a href="https://ayb.host/u/me" rel="nofollow">x</a>"#,
            URL
        ));
    }

    #[test]
    fn negative_rel_substring_only() {
        assert!(!html_has_rel_me_link(
            r#"<a href="https://ayb.host/u/me" rel="metoo">x</a>"#,
            URL
        ));
    }

    #[test]
    fn positive_unquoted_rel() {
        assert!(html_has_rel_me_link(
            r#"<a rel=me href="https://ayb.host/u/me">x</a>"#,
            URL
        ));
    }

    #[test]
    fn negative_rel_hyphenated_me() {
        // `me-author` is a single rel token, not the `me` keyword: the
        // pre-filter regex would accept (\b at `-`), strict check rejects.
        assert!(!html_has_rel_me_link(
            r#"<a href="https://ayb.host/u/me" rel="me-author">x</a>"#,
            URL
        ));
    }

    #[test]
    fn negative_href_and_rel_on_different_tags() {
        assert!(!html_has_rel_me_link(
            r#"<a href="https://ayb.host/u/me">x</a><a rel="me">y</a>"#,
            URL
        ));
    }
}
