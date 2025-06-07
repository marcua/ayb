use crate::http::structs::EntityDatabasePath;
use crate::server::config::AybConfig;
use crate::server::ui_endpoints::auth::{authentication_details, init_ayb_client};
use crate::server::ui_endpoints::templates::ok_response;
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

    // Get share list if user can manage the database
    let share_list = if database_response.can_manage_database {
        match client.share_list(entity_slug, database_slug).await {
            Ok(shares) => Some(shares.sharing_entries),
            Err(_) => None,
        }
    } else {
        None
    };

    let mut context = tera::Context::new();
    context.insert("entity", entity_slug);
    context.insert("database", database_slug);
    context.insert("database_type", &database_response.database_type);
    context.insert(
        "can_manage_database",
        &database_response.can_manage_database,
    );
    context.insert(
        "highest_query_access_level",
        &database_response.highest_query_access_level,
    );
    context.insert(
        "public_sharing_level",
        &database_response.public_sharing_level,
    );
    context.insert(
        "logged_in_entity",
        &authentication_details(&req).map(|details| details.entity),
    );
    context.insert("share_list", &share_list);

    ok_response("database.html", &context)
}
