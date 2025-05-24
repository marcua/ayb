use crate::ayb_db::models::PublicSharingLevel;
use crate::http::structs::EntityPath;
use crate::server::config::AybConfig;
use crate::server::ui_endpoints::auth::{authentication_details, init_ayb_client};
use crate::server::ui_endpoints::templates::render;
use actix_web::{get, web, HttpRequest, HttpResponse, Result};

#[get("/{entity}")]
pub async fn entity_details(
    req: HttpRequest,
    path: web::Path<EntityPath>,
    ayb_config: web::Data<AybConfig>,
) -> Result<HttpResponse> {
    let entity_slug = &path.entity.to_lowercase();
    let client = init_ayb_client(&ayb_config, &req);

    // Get entity details using the API client
    let entity_response = match client.entity_details(entity_slug).await {
        Ok(response) => response,
        Err(_) => return Ok(HttpResponse::NotFound().body("Entity not found")),
    };

    let name = entity_response
        .profile
        .display_name
        .as_deref()
        .unwrap_or(&entity_response.slug);
    
    let mut context = tera::Context::new();
    context.insert("name", name);
    context.insert("entity", entity_slug);
    context.insert("description", &entity_response.profile.description.unwrap_or_default());
    context.insert("organization", &entity_response.profile.organization);
    context.insert("location", &entity_response.profile.location);
    context.insert("links", &entity_response.profile.links);
    context.insert("can_create_database", &entity_response.permissions.can_create_database);
    context.insert("databases", &entity_response.databases);
    
    // Add sharing level values for the template
    context.insert("no_access", PublicSharingLevel::NoAccess.to_str());
    context.insert("fork", PublicSharingLevel::Fork.to_str());
    context.insert("read_only", PublicSharingLevel::ReadOnly.to_str());
    
    context.insert("logged_in_entity", &authentication_details(&req).map(|details| details.entity));

    Ok(HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(render("entity_details.html", &context)))
}
