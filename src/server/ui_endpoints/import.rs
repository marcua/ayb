use crate::http::structs::EntityDatabasePath;
use crate::server::config::AybConfig;
use crate::server::ui_endpoints::auth::init_ayb_client;
use crate::server::ui_endpoints::templates::{error_snippet, success_snippet};
use actix_multipart::form::{tempfile::TempFile, MultipartForm};
use actix_web::{post, web, HttpRequest, HttpResponse, Result};

#[derive(MultipartForm)]
pub struct ImportDatabaseForm {
    #[multipart(rename = "database")]
    database: TempFile,
}

#[post("/{entity}/{database}/import")]
pub async fn import_database(
    req: HttpRequest,
    path: web::Path<EntityDatabasePath>,
    MultipartForm(form): MultipartForm<ImportDatabaseForm>,
    ayb_config: web::Data<AybConfig>,
) -> Result<HttpResponse> {
    let entity_slug = &path.entity.to_lowercase();
    let database_slug = &path.database.to_lowercase();

    if form.database.size == 0 {
        return error_snippet("Missing file", "Please choose a database file to upload.");
    }

    let client = init_ayb_client(&ayb_config, &req);

    match client
        .import_database(entity_slug, database_slug, form.database.file.path())
        .await
    {
        Ok(_) => success_snippet(&format!(
            "Database {entity_slug}/{database_slug} successfully replaced with the uploaded file."
        )),
        Err(err) => error_snippet("Error importing database", &format!("{err}")),
    }
}
