use crate::ayb_db::models::{InstantiatedDatabase, InstantiatedEntity};

pub fn can_create_database(
    authenticated_entity: &InstantiatedEntity,
    desired_entity: &InstantiatedEntity,
) -> bool {
    // An entity/user can only create databases on itself (for now)
    return authenticated_entity.id == desired_entity.id;
}

pub fn can_query(
    authenticated_entity: &InstantiatedEntity,
    database: &InstantiatedDatabase,
) -> bool {
    // An entity/user can only query its own databases (for now)
    return authenticated_entity.id == database.entity_id;
}
