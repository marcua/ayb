use lazy_static::lazy_static;
use tera::{Context, Tera};

lazy_static! {
    pub static ref TEMPLATES: Tera = {
        let tera = match Tera::new("src/server/ui_endpoints/templates/**/*.html") {
            Ok(t) => t,
            Err(e) => {
                println!("Parsing error(s): {}", e);
                ::std::process::exit(1);
            }
        };
        tera
    };
}

pub fn render(template_name: &str, context: &Context) -> String {
    TEMPLATES
        .render(template_name, context)
        .unwrap_or_else(|e| {
            eprintln!("Template error: {}", e);
            format!("Error rendering template: {}", e)
        })
}

