use crate::ayb_db::models::{
    DBType, EntityType, InstantiatedDatabase as PersistedDatabase,
    InstantiatedEntity as PersistedEntity,
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
pub struct AybConfigAuthentication {
    pub fernet_key: String,
    pub token_expiration_seconds: u64,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct AybConfigEmail {
    pub from: String,
    pub reply_to: String,
    pub smtp_host: String,
    pub smtp_port: u16,
    pub smtp_username: String,
    pub smtp_password: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct AybConfig {
    pub host: String,
    pub port: u16,
    pub database_url: String,
    pub data_path: String,
    pub e2e_testing: Option<bool>,
    pub authentication: AybConfigAuthentication,
    pub email: AybConfigEmail,
}

impl AybConfig {
    pub fn e2e_testing_on(&self) -> bool {
        match self.e2e_testing {
            Some(v) => v,
            None => false,
        }
    }
}

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

#[derive(Debug, Serialize, Deserialize)]
pub struct AuthenticationDetails {
    pub version: u16,
    pub entity: String,
    pub entity_type: i16,
    pub email_address: String,
    pub create_api_token: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct APIToken {
    pub token: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EmptyResponse {}
