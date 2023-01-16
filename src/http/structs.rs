use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct EntityDatabasePath {
    pub entity: String,
    pub database: String,
}

#[derive(Serialize, Deserialize)]
pub struct EntityPath {
    pub entity: String,
}
