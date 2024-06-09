use crate::ayb_db::models::{InstantiatedDatabase, InstantiatedEntity};

pub fn can_create_database(
    authenticated_entity: &InstantiatedEntity,
    desired_entity: &InstantiatedEntity,
) -> bool {
    // An entity/user can only create databases on itself (for now)
    authenticated_entity.id == desired_entity.id
}

pub fn can_query(
    authenticated_entity: &InstantiatedEntity,
    database: &InstantiatedDatabase,
) -> bool {
    // An entity/user can only query its own databases (for now)
    authenticated_entity.id == database.entity_id
}

pub fn can_manage_snapshots(
    authenticated_entity: &InstantiatedEntity,
    database: &InstantiatedDatabase,
) -> bool {
    // An entity/user can only manage snapshots on its own databases (for now)
    authenticated_entity.id == database.entity_id
}
