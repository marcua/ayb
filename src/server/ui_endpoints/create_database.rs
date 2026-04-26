use crate::ayb_db::models::{DBType, PublicSharingLevel};
use crate::http::structs::EntityPath;
use crate::server::config::AybConfig;
use crate::server::ui_endpoints::auth::init_ayb_client;
use crate::server::ui_endpoints::templates::error_snippet;
use actix_multipart::form::{tempfile::TempFile, text::Text, MultipartForm};
use actix_web::{post, web, HttpRequest, HttpResponse, Result};
use std::str::FromStr;

#[derive(MultipartForm)]
pub struct CreateDatabaseForm {
    database_slug: Text<String>,
    public_sharing_level: Text<String>,
    #[multipart(rename = "database")]
    database: Option<TempFile>,
}

#[post("/{entity}/create_database")]
pub async fn create_database(
    req: HttpRequest,
    path: web::Path<EntityPath>,
    MultipartForm(form): MultipartForm<CreateDatabaseForm>,
    ayb_config: web::Data<AybConfig>,
) -> Result<HttpResponse> {
    let entity_slug = &path.entity.to_lowercase();
    let database_slug = form.database_slug.to_lowercase();
    let public_sharing_level = match PublicSharingLevel::from_str(&form.public_sharing_level) {
        Ok(level) => level,
        Err(err) => return error_snippet("Error creating database", &format!("{err}")),
    };

    let client = init_ayb_client(&ayb_config, &req);
    let seed_path = form.database.as_ref().map(|tmp| tmp.file.path().to_owned());

    match client
        .create_database(
            entity_slug,
            &database_slug,
            &DBType::Sqlite,
            &public_sharing_level,
            seed_path.as_deref(),
        )
        .await
    {
        Ok(_) => {
            let redirect_url = format!("/{entity_slug}/{database_slug}");
            Ok(HttpResponse::Ok()
                .append_header(("HX-Redirect", redirect_url))
                .finish())
        }
        Err(err) => error_snippet("Error creating database", &format!("{err}")),
    }
}
