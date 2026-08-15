use crate::ayb_db::db_interfaces::AybDb;
use crate::ayb_db::models::{APIToken, DBType, InstantiatedEntity};
use crate::error::AybError;
use crate::hosted_db::engine_for;
use crate::hosted_db::paths::{current_database_path, database_export_path};
use crate::http::structs::EntityDatabasePath;
use crate::server::config::AybConfig;
use crate::server::permissions::highest_query_access_level;
use crate::server::utils::unwrap_authenticated_entity;
use actix_files::NamedFile;
use actix_web::http::header::{ContentDisposition, DispositionParam, DispositionType};
use actix_web::mime;
use actix_web::{get, web, HttpRequest, HttpResponse};
use std::fs;

#[get(
    "/{entity}/{database}/export",
    wrap = "actix_web_httpauth::middleware::HttpAuthentication::bearer(crate::server::server_runner::entity_validator)"
)]
async fn export(
    req: HttpRequest,
    path: web::Path<EntityDatabasePath>,
    ayb_db: web::Data<Box<dyn AybDb>>,
    ayb_config: web::Data<AybConfig>,
    authenticated_entity: Option<web::ReqData<InstantiatedEntity>>,
    api_token: Option<web::ReqData<APIToken>>,
) -> Result<HttpResponse, AybError> {
    let entity_slug = &path.entity.to_lowercase();
    let database_slug = &path.database;
    let database = ayb_db.get_database(entity_slug, database_slug).await?;
    let authenticated_entity = unwrap_authenticated_entity(&authenticated_entity)?;
    let token_ref = api_token.as_ref().map(|t| t.clone().into_inner());
    let token = token_ref.as_ref();

    // Read access (read-only or read-write) is sufficient to export.
    let access_level =
        highest_query_access_level(&authenticated_entity, &database, token, &ayb_db).await?;
    if access_level.is_none() {
        return Err(AybError::Other {
            message: format!(
                "Authenticated entity {} can't export database {}/{}",
                authenticated_entity.slug, entity_slug, database_slug
            ),
        });
    }

    let db_type = DBType::try_from(database.db_type)?;
    let db_path = current_database_path(entity_slug, database_slug, &ayb_config.data_path)?;
    let temp_dir = database_export_path(entity_slug, database_slug, &ayb_config.data_path)?;
    let temp_path = temp_dir.join(database_slug);

    // An export is exactly a snapshot that never reaches S3: the same
    // engine-produced consistent copy, generated fresh for this request
    // rather than read back from the last scheduled backup.
    match engine_for(&db_type).create_snapshot(&db_path, &temp_path) {
        Ok(()) => stream_and_clean_up(&req, &temp_path, &temp_dir, database_slug),
        Err(err) => {
            let _ = fs::remove_dir_all(&temp_dir);
            Err(err)
        }
    }
}

fn stream_and_clean_up(
    req: &HttpRequest,
    file_path: &std::path::Path,
    temp_dir: &std::path::Path,
    download_name: &str,
) -> Result<HttpResponse, AybError> {
    let file = std::fs::File::open(file_path)?;

    // Unlink the file (Unix: handle keeps inode alive until streamed
    // out; on other platforms the file lingers until next restart).
    let _ = fs::remove_file(file_path);
    let _ = fs::remove_dir(temp_dir);

    let named_file = NamedFile::from_file(file, download_name)?
        .set_content_type(mime::APPLICATION_OCTET_STREAM)
        .set_content_disposition(ContentDisposition {
            disposition: DispositionType::Attachment,
            parameters: vec![DispositionParam::Filename(download_name.to_string())],
        });
    Ok(named_file.into_response(req))
}
