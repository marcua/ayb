use clap::ValueEnum;
use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};
use sqlx::FromRow;
use std::fmt;

#[derive(
    Serialize_repr, Deserialize_repr, Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, ValueEnum,
)]
#[repr(i16)]
pub enum DBType {
    Sqlite = 0,
    Duckdb = 1,
}

impl fmt::Display for DBType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl DBType {
    pub fn from_i16(value: i16) -> DBType {
        match value {
            0 => DBType::Sqlite,
            1 => DBType::Duckdb,
            _ => panic!("Unknown value: {}", value),
        }
    }

    pub fn from_str(value: &str) -> DBType {
        match value {
            "sqlite" => DBType::Sqlite,
            "duckdb" => DBType::Duckdb,
            _ => panic!("Unknown value: {}", value),
        }
    }

    pub fn to_str(&self) -> &str {
        match self {
            DBType::Sqlite => "sqlite",
            DBType::Duckdb => "duckdb",
        }
    }
}

#[derive(
    Serialize_repr, Deserialize_repr, Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, ValueEnum,
)]
#[repr(i16)]
pub enum EntityType {
    User = 0,
    Organization = 1,
}

impl fmt::Display for EntityType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl EntityType {
    pub fn from_i16(value: i16) -> EntityType {
        match value {
            0 => EntityType::User,
            1 => EntityType::Organization,
            _ => panic!("Unknown value: {}", value),
        }
    }

    pub fn from_str(value: &str) -> EntityType {
        match value {
            "user" => EntityType::User,
            "organization" => EntityType::Organization,
            _ => panic!("Unknown value: {}", value),
        }
    }

    pub fn to_str(&self) -> &str {
        match self {
            EntityType::User => "user",
            EntityType::Organization => "organization",
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Database {
    pub entity_id: i32,
    pub slug: String,
    pub db_type: i16,
}

#[derive(Debug, FromRow, Serialize, Deserialize)]
pub struct InstantiatedDatabase {
    pub id: i32,
    pub entity_id: i32,
    pub slug: String,
    pub db_type: i16,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Entity {
    pub slug: String,
    pub entity_type: i16,
}

#[derive(Debug, FromRow, Serialize, Deserialize)]
pub struct InstantiatedEntity {
    pub id: i32,
    pub slug: String,
    pub entity_type: i16,
}
