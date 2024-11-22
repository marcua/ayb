use crate::ayb_db::db_interfaces::AybDb;
use crate::ayb_db::models::{
    EntityDatabaseSharingLevel, InstantiatedDatabase, InstantiatedEntity, PublicSharingLevel,
};
use crate::error::AybError;
use crate::hosted_db::QueryMode;
use actix_web::web;

fn is_owner(authenticated_entity: &InstantiatedEntity, database: &InstantiatedDatabase) -> bool {
    authenticated_entity.id == database.entity_id
}

pub fn can_create_database(
    authenticated_entity: &InstantiatedEntity,
    desired_entity: &InstantiatedEntity,
) -> bool {
    authenticated_entity.id == desired_entity.id
}

pub fn can_discover_database(
    authenticated_entity: &InstantiatedEntity,
    database: &InstantiatedDatabase,
) -> Result<bool, AybError> {
    let public_sharing_level = PublicSharingLevel::try_from(database.public_sharing_level)?;
    Ok(is_owner(authenticated_entity, database)
        || public_sharing_level == PublicSharingLevel::ReadOnly
        || public_sharing_level == PublicSharingLevel::Fork)
}

pub async fn can_manage_database(
    authenticated_entity: &InstantiatedEntity,
    database: &InstantiatedDatabase,
    ayb_db: &web::Data<Box<dyn AybDb>>,
) -> Result<bool, AybError> {
    if is_owner(authenticated_entity, database) {
        return Ok(true);
    }

    let permission = ayb_db
        .get_entity_database_permission(authenticated_entity, database)
        .await?;
    match permission {
        Some(permission) => match EntityDatabaseSharingLevel::try_from(permission.sharing_level)? {
            EntityDatabaseSharingLevel::Manager => Ok(true),
            _ => Ok(false),
        },
        None => Ok(false),
    }
}

pub async fn highest_query_access_level(
    authenticated_entity: &InstantiatedEntity,
    database: &InstantiatedDatabase,
    ayb_db: &web::Data<Box<dyn AybDb>>,
) -> Result<Option<QueryMode>, AybError> {
    if is_owner(authenticated_entity, database) {
        Ok(Some(QueryMode::ReadWrite))
    } else if PublicSharingLevel::try_from(database.public_sharing_level)?
        == PublicSharingLevel::ReadOnly
    {
        Ok(Some(QueryMode::ReadOnly))
    } else {
        let permission = ayb_db
            .get_entity_database_permission(authenticated_entity, database)
            .await?;
        match permission {
            Some(permission) => {
                match EntityDatabaseSharingLevel::try_from(permission.sharing_level)? {
                    EntityDatabaseSharingLevel::Manager | EntityDatabaseSharingLevel::ReadWrite => {
                        Ok(Some(QueryMode::ReadWrite))
                    }
                    EntityDatabaseSharingLevel::ReadOnly => Ok(Some(QueryMode::ReadOnly)),
                    _ => Ok(None),
                }
            }
            None => Ok(None),
        }
    }
}

pub fn can_manage_snapshots(
    authenticated_entity: &InstantiatedEntity,
    database: &InstantiatedDatabase,
) -> bool {
    // An entity/user can only manage snapshots on its own databases (for now)
    is_owner(authenticated_entity, database)
}
