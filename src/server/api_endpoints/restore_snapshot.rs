use crate::ayb_db::db_interfaces::AybDb;
use crate::ayb_db::models::InstantiatedEntity;
use crate::error::AybError;
use crate::hosted_db::paths::{new_database_path, set_current_database_and_clean_up};
use crate::http::structs::{EmptyResponse, EntityDatabasePath};
use crate::server::config::AybConfig;
use crate::server::permissions::can_manage_database;
use crate::server::snapshots::storage::SnapshotStorage;
use crate::server::utils::unwrap_authenticated_entity;
use actix_web::{post, web, HttpResponse};

#[post("/{entity}/{database}/restore_snapshot")]
async fn restore_snapshot(
    path: web::Path<EntityDatabasePath>,
    snapshot_id: String,
    ayb_db: web::Data<Box<dyn AybDb>>,
    ayb_config: web::Data<AybConfig>,
    authenticated_entity: Option<web::ReqData<InstantiatedEntity>>,
) -> Result<HttpResponse, AybError> {
    let entity_slug = &path.entity.to_lowercase();
    let database_slug = &path.database;
    println!(
        "[SNAPSHOT_RESTORE] Restore request for {}/{} snapshot {} at {}",
        entity_slug,
        database_slug,
        snapshot_id,
        chrono::Utc::now()
    );
    let database = ayb_db.get_database(entity_slug, database_slug).await?;
    let authenticated_entity = unwrap_authenticated_entity(&authenticated_entity)?;

    if can_manage_database(&authenticated_entity, &database, &ayb_db).await? {
        if let Some(ref snapshot_config) = ayb_config.snapshots {
            // TODO(marcua): In the future, consider quiescing
            // requests to this database during the process, and
            // locking so that only one snapshot per database can be
            // restored at a time.

            // Retrieve the snapshot, move it to the active databases
            // directory, and set it as the current active database.
            let snapshot_storage = SnapshotStorage::new(snapshot_config).await?;

            // List available snapshots before attempting restore
            println!("[SNAPSHOT_RESTORE] Listing available snapshots before restore attempt");
            let available_snapshots = snapshot_storage
                .list_snapshots(entity_slug, database_slug)
                .await?;
            println!(
                "[SNAPSHOT_RESTORE] Found {} available snapshots:",
                available_snapshots.len()
            );
            for (i, snap) in available_snapshots.iter().enumerate() {
                println!(
                    "  [{}] {} (last modified: {})",
                    i, snap.snapshot_id, snap.last_modified_at
                );
            }

            let snapshot_exists = available_snapshots
                .iter()
                .any(|s| s.snapshot_id == snapshot_id);
            println!(
                "[SNAPSHOT_RESTORE] Target snapshot {} exists: {}",
                snapshot_id, snapshot_exists
            );

            let db_path = &new_database_path(entity_slug, database_slug, &ayb_config.data_path)?;
            println!(
                "[SNAPSHOT_RESTORE] Attempting to retrieve snapshot to {:?}",
                db_path
            );

            snapshot_storage
                .retrieve_snapshot(entity_slug, database_slug, &snapshot_id, db_path)
                .await?;

            println!(
                "[SNAPSHOT_RESTORE] Successfully retrieved snapshot, setting as current database"
            );
            set_current_database_and_clean_up(db_path)?;
            println!(
                "[SNAPSHOT_RESTORE] Restore completed successfully at {}",
                chrono::Utc::now()
            );
        }
        Ok(HttpResponse::Ok().json(EmptyResponse {}))
    } else {
        Err(AybError::Other {
            message: format!(
                "Authenticated entity {} can't manage snapshots on database {}/{}",
                authenticated_entity.slug, entity_slug, database_slug
            ),
        })
    }
}
