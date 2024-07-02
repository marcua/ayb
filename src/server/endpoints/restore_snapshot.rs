use crate::ayb_db::db_interfaces::AybDb;
use crate::ayb_db::models::InstantiatedEntity;
use crate::error::AybError;
use crate::hosted_db::paths::{
    new_database_path, pathbuf_to_parent, set_current_database_and_clean_up,
};
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
            // TODO(marcua): In the future, consider quiescing
            // requests to this database during the process, and
            // locking so that only one snapshot per database can be
            // restored at a time.

            // Retrieve the snapshot, move it to the active databases
            // directory, and set it as the current active database.
            let snapshot_storage = SnapshotStorage::new(snapshot_config).await?;
            // TODO(marcua): Consider passing a path to
            // retrieve_snapshot so it creates the file there.
            let snapshot_path = pathbuf_to_parent(
                &snapshot_storage
                    .retrieve_snapshot(
                        entity_slug,
                        database_slug,
                        &snapshot_id,
                        &ayb_config.data_path,
                    )
                    .await?,
            )?;
            let db_path = &new_database_path(&entity_slug, &database_slug, &ayb_config.data_path)?;
            rename(snapshot_path, db_path.clone())?;
            set_current_database_and_clean_up(&db_path)?;
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
