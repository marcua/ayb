use crate::error::AybError;
use clap::ValueEnum;
use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};
use sqlx::FromRow;
use std::str::FromStr;

macro_rules! try_from_i16 {
    ($struct:ident, { $($left:literal => $right:expr),+ }) => {
        impl TryFrom<i16> for $struct {
            type Error = AybError;

            fn try_from(value: i16) -> Result<Self, Self::Error> {
                match value {
                    $($left => Ok($right),)*
                    _ => Err(Self::Error::Other {
                        message: format!("Unknown value: {}", value),
                    }),
                }
            }
        }
    };
}

macro_rules! from_str {
    ($struct:ident, { $($left:literal => $right:expr),+ }) => {
        impl FromStr for $struct {
            type Err = AybError;

            fn from_str(value: &str) -> Result<Self, Self::Err> {
                match value {
                    $($left => Ok($right),)*
                    _ => Err(Self::Err::Other {
                        message: format!("Unknown value: {}", value),
                    }),
                }
            }
        }
    };
}

#[derive(
    Serialize_repr, Deserialize_repr, Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, ValueEnum,
)]
#[repr(i16)]
pub enum DBType {
    Sqlite = 0,
    Duckdb = 1,
}

from_str!(DBType, {
    "sqlite" => DBType::Sqlite,
    "duckdb" => DBType::Duckdb
});

try_from_i16!(DBType, {
    0 => DBType::Sqlite,
    1 => DBType::Duckdb
});

impl DBType {
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

from_str!(EntityType, {
    "user" => EntityType::User,
    "organization" => EntityType::Organization
});

try_from_i16!(EntityType, {
    0 => EntityType::User,
    1 => EntityType::Organization
});

impl EntityType {
    pub fn to_str(&self) -> &str {
        match self {
            EntityType::User => "user",
            EntityType::Organization => "organization",
        }
    }
}

#[derive(
    Serialize_repr, Deserialize_repr, Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, ValueEnum,
)]
#[repr(i16)]
pub enum AuthenticationMethodType {
    Email = 0,
}

from_str!(AuthenticationMethodType, {
    "email" => AuthenticationMethodType::Email
});

try_from_i16!(AuthenticationMethodType, {
    0 => AuthenticationMethodType::Email
});

impl AuthenticationMethodType {
    pub fn to_str(&self) -> &str {
        match self {
            AuthenticationMethodType::Email => "email",
        }
    }
}

#[derive(
    Serialize_repr, Deserialize_repr, Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, ValueEnum,
)]
#[repr(i16)]
pub enum AuthenticationMethodStatus {
    Verified = 0,
    Revoked = 1,
}

from_str!(AuthenticationMethodStatus, {
    "verified" => AuthenticationMethodStatus::Verified,
    "revoked" => AuthenticationMethodStatus::Revoked
});

try_from_i16!(AuthenticationMethodStatus, {
    0 => AuthenticationMethodStatus::Verified,
    1 => AuthenticationMethodStatus::Revoked
});

impl AuthenticationMethodStatus {
    pub fn to_str(&self) -> &str {
        match self {
            AuthenticationMethodStatus::Verified => "verified",
            AuthenticationMethodStatus::Revoked => "revoked",
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
    pub display_name: Option<String>,
    pub description: Option<String>,
    pub organization: Option<String>,
    pub location: Option<String>,
    pub links: Option<Vec<Link>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PartialEntity {
    pub display_name: Option<Option<String>>,
    pub description: Option<Option<String>>,
    pub organization: Option<Option<String>>,
    pub location: Option<Option<String>>,
    pub links: Option<Option<Vec<Link>>>,
}

impl Default for PartialEntity {
    fn default() -> Self {
        Self::new()
    }
}

impl PartialEntity {
    pub fn new() -> Self {
        Self {
            display_name: None,
            description: None,
            organization: None,
            location: None,
            links: None,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Link {
    pub url: String,
    pub verified: bool,
}

#[derive(Clone, Debug, FromRow, Serialize, Deserialize)]
pub struct InstantiatedEntity {
    pub id: i32,
    pub slug: String,
    pub entity_type: i16,
    pub display_name: Option<String>,
    pub description: Option<String>,
    pub organization: Option<String>,
    pub location: Option<String>,
    pub links: Option<sqlx::types::Json<Vec<Link>>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AuthenticationMethod {
    pub entity_id: i32,
    pub method_type: i16,
    pub status: i16,
    pub email_address: String,
}

#[derive(Debug, FromRow, Serialize, Deserialize)]
pub struct InstantiatedAuthenticationMethod {
    pub id: i32,
    pub entity_id: i32,
    pub method_type: i16,
    pub status: i16,
    pub email_address: String,
}

#[derive(
    Serialize_repr, Deserialize_repr, Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, ValueEnum,
)]
#[repr(i16)]
pub enum APITokenStatus {
    Active = 0,
    Revoked = 1,
}

from_str!(APITokenStatus, {
    "active" => APITokenStatus::Active,
    "revoked" => APITokenStatus::Revoked
});

try_from_i16!(APITokenStatus, {
    0 => APITokenStatus::Active,
    1 => APITokenStatus::Revoked
});

impl APITokenStatus {
    pub fn to_str(&self) -> &str {
        match self {
            APITokenStatus::Active => "active",
            APITokenStatus::Revoked => "revoked",
        }
    }
}

#[derive(Debug, FromRow, Serialize, Deserialize)]
pub struct APIToken {
    pub entity_id: i32,
    pub short_token: String,
    pub hash: String,
    pub status: i16,
}
