use crate::stacks_db::models::{
    DBType, EntityType, InstantiatedDatabase as PersistedDatabase,
    InstantiatedEntity as PersistedEntity,
};

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct Database {
    pub entity: String,
    pub database: String,
    pub database_type: String,
}

impl Database {
    pub fn from_persisted(entity: &PersistedEntity, database: &PersistedDatabase) -> Database {
        Database {
            entity: entity.slug.clone(),
            database: database.slug.clone(),
            database_type: DBType::from_i16(database.db_type).to_str().to_string(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Entity {
    pub entity: String,
    pub entity_type: String,
}

impl Entity {
    pub fn from_persisted(entity: &PersistedEntity) -> Entity {
        Entity {
            entity: entity.slug.clone(),
            entity_type: EntityType::from_i16(entity.entity_type)
                .to_str()
                .to_string(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct EntityDatabasePath {
    pub entity: String,
    pub database: String,
}

#[derive(Serialize, Deserialize)]
pub struct EntityPath {
    pub entity: String,
}
