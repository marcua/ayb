use crate::ayb_db::models::{DBType, PublicSharingLevel};
use crate::http::structs::EntityPath;
use crate::server::config::AybConfig;
use crate::server::ui_endpoints::auth::init_ayb_client;
use crate::server::ui_endpoints::templates::error_snippet;
use actix_web::{post, web, HttpRequest, HttpResponse, Result};
use serde::Deserialize;
use std::str::FromStr;

#[derive(Deserialize)]
pub struct CreateDatabaseRequest {
    database_slug: String,
    public_sharing_level: String,
}

#[post("/{entity}/create_database")]
pub async fn create_database(
    req: HttpRequest,
    path: web::Path<EntityPath>,
    form: web::Form<CreateDatabaseRequest>,
    ayb_config: web::Data<AybConfig>,
) -> Result<HttpResponse> {
    let entity_slug = &path.entity.to_lowercase();
    let database_slug = &form.database_slug.to_lowercase();

    let public_sharing_level = PublicSharingLevel::from_str(&form.public_sharing_level)?;

    let client = init_ayb_client(&ayb_config, &req);

    // Create the database using the API client
    match client
        .create_database(
            entity_slug,
            database_slug,
            &DBType::Sqlite,
            &public_sharing_level,
        )
        .await
    {
        Ok(_) => {
            // Redirect to the new database page
            let redirect_url = format!("/{entity_slug}/{database_slug}");
            Ok(HttpResponse::Ok()
                .append_header(("HX-Redirect", redirect_url))
                .finish())
        }
        Err(err) => error_snippet("Error creating database", &format!("{err}")),
    }
}
