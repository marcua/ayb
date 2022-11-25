use clap::ValueEnum;
use serde::{Deserialize, Serialize};
use serde_repr::{Serialize_repr, Deserialize_repr};
use std::fmt;

#[derive(Serialize_repr, Deserialize_repr, Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
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

#[derive(Serialize, Deserialize)]
pub struct Database {
    pub owner_id: i32,
    pub slug: String,
    pub db_type: DBType
}

#[derive(Serialize, Deserialize)]
pub struct DatabaseOwner {
    pub id: i32,
    pub slug: String
}
