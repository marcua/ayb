use clap::ValueEnum;
use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};
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
}

#[derive(Serialize, Deserialize)]
pub struct Database {
    pub owner_id: i32,
    pub slug: String,
    pub db_type: i16,
}

#[derive(Serialize, Deserialize)]
pub struct InstantiatedDatabase {
    pub id: i32,
    pub owner_id: i32,
    pub slug: String,
    pub db_type: i16,
}

#[derive(Serialize, Deserialize)]
pub struct DatabaseOwner {
    pub slug: String,
}

#[derive(Serialize, Deserialize)]
pub struct InstantiatedDatabaseOwner {
    pub id: i32,
    pub slug: String,
}
