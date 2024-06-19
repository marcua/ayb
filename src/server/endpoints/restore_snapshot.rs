use crate::ayb_db::db_interfaces::AybDb;
use crate::ayb_db::models::InstantiatedEntity;
use crate::error::AybError;
use crate::hosted_db::paths::{database_path, pathbuf_to_parent};
use crate::http::structs::{EmptyResponse, EntityDatabasePath};
use crate::server::config::AybConfig;
use crate::server::permissions::can_manage_snapshots;
use crate::server::snapshots::storage::SnapshotStorage;
use crate::server::utils::unwrap_authenticated_entity;
use actix_web::{post, web, HttpResponse};
use std::fs::rename;

#[post("/v1/{entity}/{database}/restore_snapshot")]
async fn restore_snapshot(
    path: web::Path<EntityDatabasePath>,
    snapshot_id: String,
    ayb_db: web::Data<Box<dyn AybDb>>,
    ayb_config: web::Data<AybConfig>,
    authenticated_entity: Option<web::ReqData<InstantiatedEntity>>,
) -> Result<HttpResponse, AybError> {
    let entity_slug = &path.entity.to_lowercase();
    let database_slug = &path.database;
    let database = ayb_db.get_database(entity_slug, database_slug).await?;
    let authenticated_entity = unwrap_authenticated_entity(&authenticated_entity)?;
    
    if can_manage_snapshots(&authenticated_entity, &database) {
        if let Some(ref snapshot_config) = ayb_config.snapshots {
            // TODO(marcua): In the future, consider quiesting
            // requests to this database during the process, and
            // locking so that only one snapshot per database can be
            // restored at a time.

            let snapshot_storage = SnapshotStorage::new(snapshot_config).await?;
            let snapshot_path = snapshot_storage
                .retrieve_snapshot(
                    entity_slug,
                    database_slug,
                    &snapshot_id,
                    &ayb_config.data_path,
                )
                .await?;
            let db_path =
                database_path(&entity_slug, &database_slug, &ayb_config.data_path, false)?;
            // Atomically rename the directory holding the restored
            // snapshot file to the directory holding the active
            // database.
            rename(
                pathbuf_to_parent(&snapshot_path)?,
                pathbuf_to_parent(&db_path)?,
            )?;
        }
        Ok(HttpResponse::Ok().json(EmptyResponse {}))
    } else {
        Err(AybError::Other {
            message: format!(
                "Authenticated entity {} can not manage snapshots on database {}/{}",
                authenticated_entity.slug, entity_slug, database_slug
            ),
        })
    }
}
