use crate::ayb_db::db_interfaces::AybDb;
use crate::ayb_db::models::{DBType, Database, InstantiatedEntity, PublicSharingLevel};
use std::str::FromStr;

use crate::error::AybError;

use crate::hosted_db::daemon_registry::DaemonRegistry;
use crate::hosted_db::paths::{
    instantiated_new_database_path, pathbuf_to_parent, set_current_database_and_clean_up,
};
use crate::hosted_db::sqlite::query_sqlite;
use crate::hosted_db::QueryMode;
use crate::http::structs::{Database as APIDatabase, EntityDatabasePath};
use crate::server::config::AybConfig;
use crate::server::permissions::can_create_database;
use crate::server::utils::{get_required_header, unwrap_authenticated_entity};
use actix_multipart::form::{tempfile::TempFile, MultipartForm};
use actix_web::{post, web, HttpRequest, HttpResponse};
use std::fs;
use std::path::Path;

#[derive(MultipartForm)]
pub struct CreateDatabaseForm {
    #[multipart(rename = "database")]
    pub database: Option<TempFile>,
}

#[post("/{entity}/{database}/create")]
async fn create_database(
    path: web::Path<EntityDatabasePath>,
    req: HttpRequest,
    form: Option<MultipartForm<CreateDatabaseForm>>,
    ayb_db: web::Data<Box<dyn AybDb>>,
    ayb_config: web::Data<AybConfig>,
    daemon_registry: web::Data<DaemonRegistry>,
    authenticated_entity: Option<web::ReqData<InstantiatedEntity>>,
) -> Result<HttpResponse, AybError> {
    let entity_slug = &path.entity;

    let entity = ayb_db.get_entity_by_slug(entity_slug).await?;
    let db_type_header = get_required_header(&req, "db-type")?;
    let db_type = DBType::from_str(&db_type_header)?;
    let public_sharing_level = get_required_header(&req, "public-sharing-level")?;
    let database = Database {
        entity_id: entity.id,
        slug: path.database.clone(),
        db_type: db_type as i16,
        public_sharing_level: PublicSharingLevel::from_str(&public_sharing_level)? as i16,
    };
    let authenticated_entity = unwrap_authenticated_entity(&authenticated_entity)?;
    if !can_create_database(&authenticated_entity, &entity) {
        return Err(AybError::Other {
            message: format!(
                "Authenticated entity {} can't create a database for entity {}",
                authenticated_entity.slug, entity_slug
            ),
        });
    }

    let uploaded = form.and_then(|f| f.into_inner().database);

    // Validate the upload before touching any persistent state, so a
    // bad file doesn't leave a half-created database behind.
    if let Some(ref tmp) = uploaded {
        validate_seed_file(&db_type, tmp.file.path())?;
    }

    let created_database = ayb_db.create_database(&database).await?;
    let db_path =
        instantiated_new_database_path(entity_slug, &path.database, &ayb_config.data_path)?;

    if let Some(tmp) = uploaded {
        if let Err(err) = write_seed_to_db_path(tmp, &db_path) {
            // Best-effort rollback so the user can retry cleanly. We
            // don't currently delete the DB row — that requires a
            // delete API on AybDb that doesn't exist yet — so on
            // failure here the user must contact an admin or upload a
            // valid file under the same name later.
            let _ = fs::remove_dir_all(pathbuf_to_parent(&db_path)?);
            return Err(err);
        }
    }

    set_current_database_and_clean_up(&pathbuf_to_parent(&db_path)?, &daemon_registry).await?;
    Ok(HttpResponse::Created().json(APIDatabase::from_persisted(&entity, &created_database)))
}

fn validate_seed_file(db_type: &DBType, path: &Path) -> Result<(), AybError> {
    match db_type {
        DBType::Sqlite => {
            let result = query_sqlite(
                &path.to_path_buf(),
                "PRAGMA integrity_check;",
                false,
                QueryMode::ReadOnly,
            );
            let ok = matches!(
                &result,
                Ok(r) if r.fields.len() == 1
                    && r.rows.len() == 1
                    && r.rows[0][0] == Some("ok".to_string())
            );
            if !ok {
                return Err(AybError::Other {
                    message: match result {
                        Ok(r) => format!("Uploaded file failed SQLite integrity check: {r:?}"),
                        Err(err) => format!("Uploaded file is not a valid SQLite database: {err}"),
                    },
                });
            }
            Ok(())
        }
        _ => Err(AybError::Other {
            message: format!(
                "Seeding from an uploaded file is not supported for {} databases",
                db_type.to_str()
            ),
        }),
    }
}

fn write_seed_to_db_path(uploaded: TempFile, db_path: &Path) -> Result<(), AybError> {
    // The temp file usually lives under the OS temp dir, which may be
    // a different mount than data_path. Copy (rather than rename) to
    // avoid EXDEV; the NamedTempFile drops naturally and cleans up.
    fs::copy(uploaded.file.path(), db_path)?;
    Ok(())
}
