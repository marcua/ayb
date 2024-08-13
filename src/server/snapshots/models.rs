use crate::error::AybError;
use crate::{from_str, try_from_i16};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};
use std::collections::HashMap;
use std::str::FromStr;

#[derive(Serialize_repr, Deserialize_repr, Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
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
    // Suppress clippy here, as this exact behavior (&self) -> &str is
    // allowed in `ayb_db/models.rs` and the documentation suggests
    // I'm following proper convention.
    #[allow(clippy::wrong_self_convention)]
    pub fn to_str(&self) -> &str {
        match self {
            SnapshotType::Automatic => "automatic",
            SnapshotType::Manual => "manual",
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Snapshot {
    // TODO(marcua): Eventually we'll want an InstantiatedSnapshot,
    // but haven't needed one yet. When we do, it will need this field
    // for completeness:
    // pub last_modified_at: DateTime<Utc>,

    // A blake3 hash of the snapshot directory before compressing.
    pub snapshot_id: String,
    pub snapshot_type: i16,
}

impl Snapshot {
    pub fn to_header_map(&self) -> Result<HashMap<String, String>, AybError> {
        let mut headers = HashMap::new();
        headers.insert(
            "snapshot_type".to_string(),
            SnapshotType::try_from(self.snapshot_type)?
                .to_str()
                .to_string(),
        );
        Ok(headers)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ListSnapshotResult {
    pub last_modified_at: DateTime<Utc>,
    pub snapshot_id: String,
}
