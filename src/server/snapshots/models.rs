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
    // We take two snapshots of the database. The first, a
    // `pre_snapshot_hash`, is taken of the actual database's files
    // before taking a snapshot. The second is a the `snapshot_hash`,
    // which is the hash of the snapshot's files. Because a database
    // is copied (and potentially vacuumed) in order to take a
    // snapshot, its snapshot hash might be different from that of the
    // original database from which it was copied. If a database
    // snapshot is taken and then doesn't change by the next snapshot,
    // it will have the same hash as the `pre_snapshot_hash`. If a
    // database is restored from snapshot and then doesn't change the
    // next time snapshot is taken, it will have the same hash as the
    // `snapshot_hash`.
    pub pre_snapshot_hash: String,
    pub snapshot_hash: String,
    pub database_id: i32,
    pub snapshot_type: i16,
}

impl Snapshot {
    pub fn to_header_map(&self) -> Result<HashMap<String, String>, AybError> {
        let mut headers = HashMap::new();
        headers.insert(
            "pre_snapshot_hash".to_string(),
            self.pre_snapshot_hash.clone(),
        );
        headers.insert("snapshot_hash".to_string(), self.snapshot_hash.clone());
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
pub struct InstantiatedSnapshot {
    pub created_at: DateTime<Utc>,
    // See Snapshot for a defintion of the two hash fields.
    pub pre_snapshot_hash: String,
    pub snapshot_hash: String,
    pub database_id: i32,
    pub snapshot_type: i16,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ListSnapshotResult {
    pub last_modified_at: DateTime<Utc>,
    pub name: String,
}
