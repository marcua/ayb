use crate::ayb_db::models::{InstantiatedDatabase, InstantiatedEntity, PublicSharingLevel};
use crate::error::AybError;
use crate::hosted_db::QueryMode;

fn is_owner(authenticated_entity: &InstantiatedEntity, database: &InstantiatedDatabase) -> bool {
    authenticated_entity.id == database.entity_id
}

pub fn can_create_database(
    authenticated_entity: &InstantiatedEntity,
    desired_entity: &InstantiatedEntity,
) -> bool {
    // An entity/user can only create databases on itself (for now)
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

pub fn can_manage_database(
    authenticated_entity: &InstantiatedEntity,
    database: &InstantiatedDatabase,
) -> bool {
    // An entity/user can only manage its own databases (for now)
    is_owner(authenticated_entity, database)
}

pub fn highest_query_access_level(
    authenticated_entity: &InstantiatedEntity,
    database: &InstantiatedDatabase,
) -> Result<Option<QueryMode>, AybError> {
    if is_owner(authenticated_entity, database) {
        Ok(Some(QueryMode::ReadWrite))
    } else if PublicSharingLevel::try_from(database.public_sharing_level)?
        == PublicSharingLevel::ReadOnly
    {
        Ok(Some(QueryMode::ReadOnly))
    } else {
        Ok(None)
    }
}

pub fn can_manage_snapshots(
    authenticated_entity: &InstantiatedEntity,
    database: &InstantiatedDatabase,
) -> bool {
    // An entity/user can only manage snapshots on its own databases (for now)
    is_owner(authenticated_entity, database)
}
