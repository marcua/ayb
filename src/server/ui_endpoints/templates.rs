use std::sync::OnceLock;
use tera::{Context, Tera};

// TEMPLATES is initialized on first use via the `templates` function.
static TEMPLATES: OnceLock<Tera> = OnceLock::new();

fn templates() -> &'static Tera {
    TEMPLATES.get_or_init(|| {
        Tera::new("src/server/ui_endpoints/templates/**/*.html")
            .unwrap_or_else(|e| {
                eprintln!("Parsing Tera templates failed: {}", e);
                std::process::exit(1);
            })
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
