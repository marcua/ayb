use std::sync::OnceLock;
use tera::{Context, Tera};

// Embed all template files directly into the binary
// AI: Since these are only used in the `templates` function, inline the calls to `include_str!` in that function and don't define the constants globally. AI!
const BASE_HTML: &str = include_str!("templates/base.html");
const BASE_AUTH_HTML: &str = include_str!("templates/base_auth.html");
const BASE_CONTENT_HTML: &str = include_str!("templates/base_content.html");
const CONFIRM_ERROR_HTML: &str = include_str!("templates/confirm_error.html");
const CONFIRM_SUCCESS_HTML: &str = include_str!("templates/confirm_success.html");
const DATABASE_HTML: &str = include_str!("templates/database.html");
const ENTITY_DETAILS_HTML: &str = include_str!("templates/entity_details.html");
const LOG_IN_HTML: &str = include_str!("templates/log_in.html");
const LOG_IN_CHECK_EMAIL_HTML: &str = include_str!("templates/log_in_check_email.html");
const LOG_IN_ERROR_HTML: &str = include_str!("templates/log_in_error.html");
const QUERY_RESULTS_HTML: &str = include_str!("templates/query_results.html");
const REGISTER_HTML: &str = include_str!("templates/register.html");
const REGISTER_CHECK_EMAIL_HTML: &str = include_str!("templates/register_check_email.html");
const REGISTER_ERROR_HTML: &str = include_str!("templates/register_error.html");

// TEMPLATES is initialized on first use via the `templates` function.
static TEMPLATES: OnceLock<Tera> = OnceLock::new();

fn templates() -> &'static Tera {
    TEMPLATES.get_or_init(|| {
        let mut tera = Tera::default();

        // Add all templates manually
        tera.add_raw_template("base.html", BASE_HTML).unwrap();
        tera.add_raw_template("base_auth.html", BASE_AUTH_HTML)
            .unwrap();
        tera.add_raw_template("base_content.html", BASE_CONTENT_HTML)
            .unwrap();
        tera.add_raw_template("confirm_error.html", CONFIRM_ERROR_HTML)
            .unwrap();
        tera.add_raw_template("confirm_success.html", CONFIRM_SUCCESS_HTML)
            .unwrap();
        tera.add_raw_template("database.html", DATABASE_HTML)
            .unwrap();
        tera.add_raw_template("entity_details.html", ENTITY_DETAILS_HTML)
            .unwrap();
        tera.add_raw_template("log_in.html", LOG_IN_HTML).unwrap();
        tera.add_raw_template("log_in_check_email.html", LOG_IN_CHECK_EMAIL_HTML)
            .unwrap();
        tera.add_raw_template("log_in_error.html", LOG_IN_ERROR_HTML)
            .unwrap();
        tera.add_raw_template("query_results.html", QUERY_RESULTS_HTML)
            .unwrap();
        tera.add_raw_template("register.html", REGISTER_HTML)
            .unwrap();
        tera.add_raw_template("register_check_email.html", REGISTER_CHECK_EMAIL_HTML)
            .unwrap();
        tera.add_raw_template("register_error.html", REGISTER_ERROR_HTML)
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
