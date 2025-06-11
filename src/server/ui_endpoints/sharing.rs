use crate::ayb_db::models::{EntityDatabaseSharingLevel, PublicSharingLevel};
use crate::http::structs::EntityDatabasePath;
use crate::server::config::AybConfig;
use crate::server::ui_endpoints::auth::init_ayb_client;
use crate::server::ui_endpoints::templates::{render, ok_response};
use actix_web::{get, post, web, HttpRequest, HttpResponse, Result};
use serde::Deserialize;
use std::str::FromStr;

#[derive(Deserialize)]
pub struct UpdatePublicSharingRequest {
    public_sharing_level: String,
}

#[derive(Deserialize)]
pub struct ShareWithEntityRequest {
    entity: String,
    sharing_level: String,
}

#[post("/{entity}/{database}/update_public_sharing")]
pub async fn update_public_sharing(
    req: HttpRequest,
    path: web::Path<EntityDatabasePath>,
    form: web::Form<UpdatePublicSharingRequest>,
    ayb_config: web::Data<AybConfig>,
) -> Result<HttpResponse> {
    let entity_slug = &path.entity.to_lowercase();
    let database_slug = &path.database.to_lowercase();

    let public_sharing_level = match PublicSharingLevel::from_str(&form.public_sharing_level) {
        Ok(level) => level,
        Err(_) => {
            let mut context = tera::Context::new();
            context.insert("title", "Invalid sharing level");
            context.insert("message", &format!("The sharing level '{}' is not valid.", form.public_sharing_level));
            return Ok(HttpResponse::BadRequest()
                .content_type("text/html")
                .body(render("sharing_error.html", &context)));
        }
    };

    let client = init_ayb_client(&ayb_config, &req);

    match client
        .update_database(entity_slug, database_slug, &public_sharing_level)
        .await
    {
        Ok(_) => {
            let mut context = tera::Context::new();
            context.insert("message", "Public sharing level updated successfully.");
            Ok(HttpResponse::Ok()
                .content_type("text/html")
                .body(render("sharing_success.html", &context)))
        }
        Err(err) => {
            let mut context = tera::Context::new();
            context.insert("title", "Error updating sharing level");
            context.insert("message", &format!("{}", err));
            Ok(HttpResponse::BadRequest()
                .content_type("text/html")
                .body(render("sharing_error.html", &context)))
        }
    }
}

#[get("/{entity}/{database}/permissions")]
pub async fn database_permissions(
    req: HttpRequest,
    path: web::Path<EntityDatabasePath>,
    ayb_config: web::Data<AybConfig>,
) -> Result<HttpResponse> {
    let entity_slug = &path.entity.to_lowercase();
    let database_slug = &path.database.to_lowercase();

    let client = init_ayb_client(&ayb_config, &req);

    match client
        .list_database_permissions(entity_slug, database_slug)
        .await
    {
        Ok(permissions) => {
            let mut context = tera::Context::new();
            context.insert("permissions", &permissions.permissions);

            let html = render("database_permissions.html", &context);
            Ok(HttpResponse::Ok().content_type("text/html").body(html))
        }
        Err(err) => {
            let mut context = tera::Context::new();
            context.insert("title", "Error loading permissions");
            context.insert("message", &format!("{}", err));
            Ok(HttpResponse::BadRequest()
                .content_type("text/html")
                .body(render("sharing_error.html", &context)))
        }
    }
}

#[post("/{entity}/{database}/share")]
pub async fn share_with_entity(
    req: HttpRequest,
    path: web::Path<EntityDatabasePath>,
    form: web::Form<ShareWithEntityRequest>,
    ayb_config: web::Data<AybConfig>,
) -> Result<HttpResponse> {
    let entity_slug = &path.entity.to_lowercase();
    let database_slug = &path.database.to_lowercase();
    let target_entity = &form.entity.trim().to_lowercase();

    let sharing_level = match EntityDatabaseSharingLevel::from_str(&form.sharing_level) {
        Ok(level) => level,
        Err(_) => {
            let mut context = tera::Context::new();
            context.insert("title", "Invalid sharing level");
            context.insert("message", &format!("The sharing level '{}' is not valid.", form.sharing_level));
            return Ok(HttpResponse::BadRequest()
                .content_type("text/html")
                .body(render("sharing_error.html", &context)));
        }
    };

    if target_entity.is_empty() {
        let mut context = tera::Context::new();
        context.insert("title", "Missing username");
        context.insert("message", "Please enter a username to share with.");
        return Ok(HttpResponse::BadRequest()
            .content_type("text/html")
            .body(render("sharing_error.html", &context)));
    }

    let client = init_ayb_client(&ayb_config, &req);

    match client
        .share(entity_slug, database_slug, target_entity, &sharing_level)
        .await
    {
        Ok(_) => {
            let mut context = tera::Context::new();
            context.insert("message", &format!("Database access updated for user '{}'.", target_entity));
            Ok(HttpResponse::Ok()
                .content_type("text/html")
                .body(render("sharing_success.html", &context)))
        }
        Err(err) => {
            let mut context = tera::Context::new();
            context.insert("title", "Error updating access");
            context.insert("message", &format!("{}", err));
            Ok(HttpResponse::BadRequest()
                .content_type("text/html")
                .body(render("sharing_error.html", &context)))
        }
    }
}
