use crate::ayb_db::db_interfaces::AybDb;
use crate::ayb_db::models::InstantiatedEntity;
use crate::error::AybError;
use crate::hosted_db::daemon_registry::DaemonRegistry;
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
    daemon_registry: web::Data<DaemonRegistry>,
    authenticated_entity: Option<web::ReqData<InstantiatedEntity>>,
) -> Result<HttpResponse, AybError> {
    let entity_slug = &path.entity.to_lowercase();
    let database_slug = &path.database;

    // Special handling for ayb_db metadata database restore
    if entity_slug == "__ayb__" && database_slug == "ayb" {
        // TODO(marcua): Implement proper authentication for ayb_db restore.
        // For now, restoring the ayb_db is disabled via the API for security reasons.
        // To restore ayb_db:
        // 1. Download the snapshot manually from S3
        // 2. Stop the server
        // 3. Replace the ayb_db file at the database_url path
        // 4. Restart the server
        return Err(AybError::Other {
            message: "Restoring the ayb metadata database via the API is not currently supported for security reasons. Please restore manually.".to_string(),
        });

        // The code below shows how ayb_db restoration would work once authentication is implemented:
        //
        // use crate::ayb_db::db_interfaces::{detect_ayb_db_type, AybDbType};
        // use std::fs;
        // use std::path::PathBuf;
        //
        // if detect_ayb_db_type(&ayb_config.database_url)? == AybDbType::Sqlite {
        //     if let Some(ref snapshot_config) = ayb_config.snapshots {
        //         // Extract the file path from the database_url
        //         let db_file_path = ayb_config
        //             .database_url
        //             .strip_prefix("sqlite://")
        //             .ok_or(AybError::SnapshotError {
        //                 message: "Unable to parse SQLite path from database_url".to_string(),
        //             })?;
        //         let ayb_db_path = PathBuf::from(db_file_path);
        //
        //         // Create a temporary directory for the snapshot
        //         let temp_dir = tempfile::TempDir::new()?;
        //         let snapshot_storage = SnapshotStorage::new(snapshot_config).await?;
        //
        //         // Retrieve the snapshot to the temp directory
        //         snapshot_storage
        //             .retrieve_snapshot(entity_slug, database_slug, &snapshot_id, temp_dir.path())
        //             .await?;
        //
        //         // Move the snapshot to replace the current ayb_db
        //         let mut snapshot_path = temp_dir.path().to_path_buf();
        //         snapshot_path.push(database_slug);
        //         fs::rename(snapshot_path, &ayb_db_path)?;
        //     }
        //     return Ok(HttpResponse::Ok().json(EmptyResponse {}));
        // } else {
        //     return Err(AybError::Other {
        //         message: "Only SQLite ayb_db can be restored via snapshots".to_string(),
        //     });
        // }
    }

    // Normal database restore logic
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
            let db_path = &new_database_path(entity_slug, database_slug, &ayb_config.data_path)?;

            snapshot_storage
                .retrieve_snapshot(entity_slug, database_slug, &snapshot_id, db_path)
                .await?;
            set_current_database_and_clean_up(db_path, &daemon_registry).await?;
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
