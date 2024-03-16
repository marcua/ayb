use crate::{try_from_i16, from_str};
use crate::error::AybError;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};
use std::str::FromStr;

#[derive(
    Serialize_repr, Deserialize_repr, Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord,
)]
#[repr(i16)]
pub enum SnapshotType {
    Automatic = 0,
    Manual = 1,
}

from_str!(SnapshotType, {
    "automatic" => SnapshotType::Automatic,
    "manual" => SnapshotType::Manual
});

try_from_i16!(SnapshotType, {
    0 => SnapshotType::Automatic,
    1 => SnapshotType::Manual
});

impl SnapshotType {
    pub fn to_str(&self) -> &str {
        match self {
            SnapshotType::Automatic => "automatic",
            SnapshotType::Manual => "manual",
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Snapshot {
    pub hash: String,
    pub database_id: i32,
    pub snapshot_type: i16,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct InstantiatedSnapshot {
    pub created_at: DateTime<Utc>,
    pub hash: String,
    pub database_id: i32,
    pub snapshot_type: i16,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ListSnapshotResult {
    pub last_modified_at: DateTime<Utc>,
    pub snapshot_hash: String,
}
