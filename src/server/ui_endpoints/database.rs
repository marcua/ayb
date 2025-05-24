use crate::http::structs::EntityDatabasePath;
use crate::server::config::AybConfig;
use crate::server::ui_endpoints::auth::{authentication_details, init_ayb_client};
use actix_web::{get, web, HttpRequest, HttpResponse, Result};

#[get("/{entity}/{database}")]
pub async fn database(
    req: HttpRequest,
    path: web::Path<EntityDatabasePath>,
    ayb_config: web::Data<AybConfig>,
) -> Result<HttpResponse> {
    let entity_slug = &path.entity.to_lowercase();
    let database_slug = &path.database.to_lowercase();
    let client = init_ayb_client(&ayb_config, &req);

    // Get database details using the API client
    let database_response = match client.database_details(entity_slug, database_slug).await {
        Ok(response) => response,
        Err(_) => return Ok(HttpResponse::NotFound().body("Database not found")),
    };

    let mut context = tera::Context::new();
    context.insert("entity", entity_slug);
    context.insert("database", database_slug);
    context.insert("database_type", &database_response.database_type);
    context.insert("can_manage_database", &database_response.can_manage_database);
    context.insert("highest_query_access_level", &database_response.highest_query_access_level);
    context.insert("logged_in_entity", &authentication_details(&req).map(|details| details.entity));

    Ok(HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(
            super::templates::TEMPLATES
                .render("database.html", &context)
                .unwrap_or_else(|e| {
                    eprintln!("Template error: {}", e);
                    format!("Error rendering template: {}", e)
                }),
        ))
}
