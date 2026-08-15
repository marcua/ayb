use crate::ayb_db::db_interfaces::AybDb;
use crate::ayb_db::models::{DBType, InstantiatedEntity};
use crate::error::AybError;
use crate::hosted_db::daemon_registry::DaemonRegistry;
use crate::hosted_db::paths::replace_current_database;
use crate::http::structs::{EmptyResponse, EntityDatabasePath};
use crate::server::config::AybConfig;
use crate::server::permissions::can_manage_database;
use crate::server::utils::unwrap_authenticated_entity;
use actix_multipart::form::{tempfile::TempFile, MultipartForm};
use actix_web::{post, web, HttpResponse};
use std::fs;

#[derive(MultipartForm)]
pub struct ImportDatabaseForm {
    #[multipart(rename = "database")]
    pub database: TempFile,
}

/// Replace a database's contents with an uploaded file.
///
/// This is the inverse of `export`, and the same operation as
/// `restore_snapshot` with a client-supplied file in place of a stored
/// snapshot -- so it goes through the same stage/validate/swap helper.
///
/// The engine type comes from the existing database record rather than
/// from the request, so an import can never change a database's type; it
/// can only fail validation against it.
#[post(
    "/{entity}/{database}/import",
    wrap = "actix_web_httpauth::middleware::HttpAuthentication::bearer(crate::server::server_runner::entity_validator)"
)]
async fn import_database(
    path: web::Path<EntityDatabasePath>,
    MultipartForm(form): MultipartForm<ImportDatabaseForm>,
    ayb_db: web::Data<Box<dyn AybDb>>,
    ayb_config: web::Data<AybConfig>,
    daemon_registry: web::Data<DaemonRegistry>,
    authenticated_entity: Option<web::ReqData<InstantiatedEntity>>,
) -> Result<HttpResponse, AybError> {
    let entity_slug = &path.entity.to_lowercase();
    let database_slug = &path.database;
    let database = ayb_db.get_database(entity_slug, database_slug).await?;
    let authenticated_entity = unwrap_authenticated_entity(&authenticated_entity)?;

    // Importing destroys the previous contents, so it requires the same
    // permission as restoring a snapshot.
    if !can_manage_database(&authenticated_entity, &database, &ayb_db).await? {
        return Err(AybError::Other {
            message: format!(
                "Authenticated entity {} can't import into database {}/{}",
                authenticated_entity.slug, entity_slug, database_slug
            ),
        });
    }

    let db_type = DBType::try_from(database.db_type)?;
    let uploaded = form.database.file.path().to_path_buf();
    replace_current_database(
        entity_slug,
        database_slug,
        &ayb_config.data_path,
        &db_type,
        &daemon_registry,
        |staging_dir| async move {
            // The temp file usually lives under the OS temp dir, which
            // may be a different mount than data_path. Copy (rather than
            // rename) to avoid EXDEV; the NamedTempFile drops naturally
            // and cleans up.
            fs::copy(&uploaded, staging_dir.join(database_slug))?;
            Ok(())
        },
    )
    .await?;

    Ok(HttpResponse::Ok().json(EmptyResponse {}))
}
