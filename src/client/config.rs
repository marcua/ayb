use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

#[derive(Serialize, Deserialize)]
pub struct ClientConfig {
    version: u64,
    pub authentication: HashMap<String, String>,
    pub default_url: Option<String>,
}

impl ClientConfig {
    pub fn new() -> ClientConfig {
        ClientConfig {
            version: 1,
            authentication: HashMap::new(),
            default_url: None,
        }
    }

    pub fn from_file(file_path: &PathBuf) -> Result<ClientConfig, std::io::Error> {
        if file_path.exists() {
            return Ok(serde_json::from_str(&fs::read_to_string(file_path)?)?);
        }

        Ok(ClientConfig::default())
    }

    pub fn to_file(&self, file_path: &PathBuf) -> Result<(), std::io::Error> {
        fs::create_dir_all(
            file_path
                .parent()
                .expect("unable to determine parent of ayb configuration directory"),
        )?;
        fs::write(file_path, serde_json::to_string(self)?)?;
        Ok(())
    }
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self::new()
    }
}
