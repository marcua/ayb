use actix_web::{HttpResponse, Result};
use std::sync::OnceLock;
use tera::{Context, Tera};

fn templates() -> &'static Tera {
    static TEMPLATES: OnceLock<Tera> = OnceLock::new();
    TEMPLATES.get_or_init(|| {
        let mut tera = Tera::default();

        // Add all templates manually, using include_str! calls to
        // embed the templates in compiled binary.
        tera.add_raw_template("base.html", include_str!("templates/base.html"))
            .unwrap();
        tera.add_raw_template("base_auth.html", include_str!("templates/base_auth.html"))
            .unwrap();
        tera.add_raw_template(
            "base_content.html",
            include_str!("templates/base_content.html"),
        )
        .unwrap();
        tera.add_raw_template(
            "confirm_error.html",
            include_str!("templates/confirm_error.html"),
        )
        .unwrap();
        tera.add_raw_template(
            "confirm_success.html",
            include_str!("templates/confirm_success.html"),
        )
        .unwrap();
        tera.add_raw_template("database.html", include_str!("templates/database.html"))
            .unwrap();
        tera.add_raw_template(
            "entity_details.html",
            include_str!("templates/entity_details.html"),
        )
        .unwrap();
        tera.add_raw_template("log_in.html", include_str!("templates/log_in.html"))
            .unwrap();
        tera.add_raw_template(
            "log_in_check_email.html",
            include_str!("templates/log_in_check_email.html"),
        )
        .unwrap();
        tera.add_raw_template(
            "log_in_error.html",
            include_str!("templates/log_in_error.html"),
        )
        .unwrap();
        tera.add_raw_template(
            "query_results.html",
            include_str!("templates/query_results.html"),
        )
        .unwrap();
        tera.add_raw_template("register.html", include_str!("templates/register.html"))
            .unwrap();
        tera.add_raw_template(
            "register_check_email.html",
            include_str!("templates/register_check_email.html"),
        )
        .unwrap();
        tera.add_raw_template(
            "register_error.html",
            include_str!("templates/register_error.html"),
        )
        .unwrap();
        tera.add_raw_template(
            "database_permissions.html",
            include_str!("templates/database_permissions.html"),
        )
        .unwrap();
        tera.add_raw_template(
            "sharing_success.html",
            include_str!("templates/sharing_success.html"),
        )
        .unwrap();
        tera.add_raw_template(
            "error_snippet.html",
            include_str!("templates/error_snippet.html"),
        )
        .unwrap();

        tera.build_inheritance_chains().unwrap();

        tera
    })
}

pub fn render(template_name: &str, context: &Context) -> String {
    templates()
        .render(template_name, context)
        .unwrap_or_else(|e| {
            eprintln!("Template error: {:?}", e);
            format!("Error rendering template: {}", e)
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
