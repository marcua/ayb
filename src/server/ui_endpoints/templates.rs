use lazy_static::lazy_static;
use tera::Tera;

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

pub fn base_content(title: &str, content: &str, logged_in_entity: Option<&str>) -> String {
    let mut context = tera::Context::new();
    context.insert("title", title);
    context.insert("content", content);
    context.insert("logged_in_entity", &logged_in_entity);

    TEMPLATES
        .render("base_content.html", &context)
        .unwrap_or_else(|e| {
            eprintln!("Template error: {}", e);
            format!("Error rendering template: {}", e)
        })
}
