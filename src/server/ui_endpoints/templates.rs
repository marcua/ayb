use actix_web::{HttpResponse, Result};
use chrono::{DateTime, Utc};
use std::sync::OnceLock;
use tera::{Context, Error, Kwargs, State, Tera, TeraResult};

fn templates() -> &'static Tera {
    static TEMPLATES: OnceLock<Tera> = OnceLock::new();
    TEMPLATES.get_or_init(|| {
        let mut tera = Tera::default();

        // Filters must be registered before the templates that use them.
        tera.register_filter("date", date);

        // Add all templates in one batch (so children can be added
        // before the templates they extend), using include_str! calls
        // to embed the templates in compiled binary.
        tera.add_raw_templates([
            ("base.html", include_str!("templates/base.html")),
            ("base_auth.html", include_str!("templates/base_auth.html")),
            (
                "base_content.html",
                include_str!("templates/base_content.html"),
            ),
            ("confirm.html", include_str!("templates/confirm.html")),
            (
                "confirm_error.html",
                include_str!("templates/confirm_error.html"),
            ),
            (
                "confirm_success.html",
                include_str!("templates/confirm_success.html"),
            ),
            (
                "create_database_fields.html",
                include_str!("templates/create_database_fields.html"),
            ),
            ("database.html", include_str!("templates/database.html")),
            (
                "entity_details.html",
                include_str!("templates/entity_details.html"),
            ),
            (
                "profile_fragment.html",
                include_str!("templates/profile_fragment.html"),
            ),
            ("log_in.html", include_str!("templates/log_in.html")),
            (
                "log_in_check_email.html",
                include_str!("templates/log_in_check_email.html"),
            ),
            (
                "log_in_error.html",
                include_str!("templates/log_in_error.html"),
            ),
            (
                "query_results.html",
                include_str!("templates/query_results.html"),
            ),
            ("register.html", include_str!("templates/register.html")),
            (
                "register_check_email.html",
                include_str!("templates/register_check_email.html"),
            ),
            (
                "register_error.html",
                include_str!("templates/register_error.html"),
            ),
            (
                "database_permissions.html",
                include_str!("templates/database_permissions.html"),
            ),
            (
                "database_snapshots.html",
                include_str!("templates/database_snapshots.html"),
            ),
            (
                "error_snippet.html",
                include_str!("templates/error_snippet.html"),
            ),
            (
                "success_snippet.html",
                include_str!("templates/success_snippet.html"),
            ),
            (
                "entity_tokens.html",
                include_str!("templates/entity_tokens.html"),
            ),
            (
                "oauth_authorize.html",
                include_str!("templates/oauth_authorize.html"),
            ),
            (
                "oauth_error.html",
                include_str!("templates/oauth_error.html"),
            ),
        ])
        .unwrap();

        tera
    })
}

/// Formats an RFC 3339 timestamp with a strftime `format`. Tera 2 moved
/// its `date` filter out of the core crate; this keeps Tera 1's chrono
/// formatting.
fn date(val: &str, kwargs: Kwargs, _: &State) -> TeraResult<String> {
    let format = kwargs.must_get::<&str>("format")?;
    let parsed = val
        .parse::<DateTime<Utc>>()
        .map_err(|e| Error::message(format!("Invalid date `{val}`: {e}")))?;
    Ok(parsed.format(format).to_string())
}

pub fn render(template_name: &str, context: &Context) -> String {
    templates()
        .render(template_name, context)
        .unwrap_or_else(|e| {
            eprintln!("Template error: {e:?}");
            format!("Error rendering template: {e}")
        })
}

pub fn ok_response(template_name: &str, context: &Context) -> Result<HttpResponse> {
    Ok(HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(render(template_name, context)))
}

pub fn error_snippet(title: &str, message: &str) -> Result<HttpResponse> {
    let mut context = tera::Context::new();
    context.insert("title", title);
    context.insert("message", message);
    Ok(HttpResponse::BadRequest()
        .content_type("text/html")
        .body(render("error_snippet.html", &context)))
}

pub fn success_snippet(message: &str) -> Result<HttpResponse> {
    let mut context = tera::Context::new();
    context.insert("message", message);
    Ok(HttpResponse::Ok()
        .content_type("text/html")
        .body(render("success_snippet.html", &context)))
}

#[cfg(test)]
mod tests {
    use super::{render, templates};
    use crate::server::snapshots::models::ListSnapshotResult;
    use chrono::{TimeZone, Utc};

    #[test]
    fn renders_snapshot_dates() {
        let mut context = tera::Context::new();
        context.insert(
            "snapshots",
            &vec![ListSnapshotResult {
                last_modified_at: Utc.with_ymd_and_hms(2026, 10, 20, 1, 2, 3).unwrap(),
                snapshot_id: "abc".to_string(),
            }],
        );
        let html = render("database_snapshots.html", &context);
        assert!(html.contains("2026-10-20 01:02:03 UTC"), "{html}");
    }

    #[test]
    fn renders_error_pages() {
        let empty = tera::Context::new();
        for name in [
            "confirm_error.html",
            "log_in_error.html",
            "register_error.html",
        ] {
            templates().render(name, &empty).unwrap();
        }

        let mut context = tera::Context::new();
        context.insert("title", "A title");
        context.insert("message", "A message");
        context.insert("error", "invalid_request");
        context.insert("error_description", "A description");
        for name in [
            "error_snippet.html",
            "success_snippet.html",
            "oauth_error.html",
        ] {
            templates().render(name, &context).unwrap();
        }
    }
}
