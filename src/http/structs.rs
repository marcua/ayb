use crate::ayb_db::models::{
    DBType, EntityType, InstantiatedDatabase as PersistedDatabase,
    InstantiatedEntity as PersistedEntity,
};
use clap::ValueEnum;
use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};
use std::fmt;

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
    pub smtp_username: String,
    pub smtp_password: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct AybConfig {
    pub host: String,
    pub port: u16,
    pub database_url: String,
    pub data_path: String,
    pub authentication: AybConfigAuthentication,
    pub email: AybConfigEmail,
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

#[derive(Serialize, Deserialize)]
pub struct EntityPath {
    pub entity: String,
}

#[derive(
    Serialize_repr, Deserialize_repr, Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, ValueEnum,
)]
#[repr(i16)]
pub enum AuthenticationMode {
    Register = 0,
    Login = 1,
}

impl fmt::Display for AuthenticationMode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl AuthenticationMode {
    pub fn from_i16(value: i16) -> AuthenticationMode {
        match value {
            0 => AuthenticationMode::Register,
            1 => AuthenticationMode::Login,
            _ => panic!("Unknown value: {}", value),
        }
    }

    pub fn from_str(value: &str) -> AuthenticationMode {
        match value {
            "register" => AuthenticationMode::Register,
            "login" => AuthenticationMode::Login,
            _ => panic!("Unknown value: {}", value),
        }
    }

    pub fn to_str(&self) -> &str {
        match self {
            AuthenticationMode::Register => "register",
            AuthenticationMode::Login => "login",
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AuthenticationDetails {
    pub version: u16,
    pub mode: i16,
    pub entity: String,
    pub entity_type: i16,
    pub email_address: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct APIKey {
    pub name: String,
    pub key: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EmptyResponse {}
