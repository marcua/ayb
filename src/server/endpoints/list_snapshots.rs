use crate::ayb_db::db_interfaces::AybDb;
use crate::ayb_db::models::InstantiatedEntity;

use crate::error::AybError;
use crate::http::structs::{EntityDatabasePath, SnapshotList};
use crate::server::config::AybConfig;
use crate::server::permissions::can_manage_snapshots;
use crate::server::snapshots::models::ListSnapshotResult;
use crate::server::snapshots::storage::SnapshotStorage;
use crate::server::utils::unwrap_authenticated_entity;
use actix_web::{get, web};

#[get("/v1/{entity}/{database}/list_snapshots")]
async fn list_snapshots(
    path: web::Path<EntityDatabasePath>,
    ayb_db: web::Data<Box<dyn AybDb>>,
    ayb_config: web::Data<AybConfig>,
    authenticated_entity: Option<web::ReqData<InstantiatedEntity>>,
) -> Result<web::Json<SnapshotList>, AybError> {
    let entity_slug = &path.entity.to_lowercase();
    let database_slug = &path.database;
    let database = ayb_db.get_database(entity_slug, database_slug).await?;
    let authenticated_entity = unwrap_authenticated_entity(&authenticated_entity)?;

    if can_manage_snapshots(&authenticated_entity, &database) {
        let mut recent_snapshots: Vec<ListSnapshotResult> = Vec::new();
        if let Some(ref snapshot_config) = ayb_config.snapshots {
            let snapshot_storage = SnapshotStorage::new(snapshot_config).await?;
            recent_snapshots = snapshot_storage
                .list_snapshots(&entity_slug, &database_slug)
                .await?;
        }
        Ok(web::Json(SnapshotList {
            snapshots: recent_snapshots,
        }))
    } else {
        Err(AybError::Other {
            message: format!(
                "Authenticated entity {} can not manage snapshots on database {}/{}",
                authenticated_entity.slug, entity_slug, database_slug
            ),
        })
    }
}
