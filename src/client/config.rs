use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{create_dir_all, File};
use std::io::{BufReader, BufWriter, Write};
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
            let file = File::open(file_path)?;
            let mut reader = BufReader::new(file);
            return Ok(serde_json::from_reader(&mut reader)?);
        }

        Ok(ClientConfig::new())
    }

    pub fn to_file(&self, file_path: &PathBuf) -> Result<(), std::io::Error> {
        create_dir_all(
            file_path
                .parent()
                .expect("unable to determine parent of ayb configuration directory"),
        )?;
        let file = File::create(file_path)?;
        let mut writer = BufWriter::new(file);
        serde_json::to_writer(&mut writer, &self)?;
        writer.flush()?;
        Ok(())
    }
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self::new()
    }
}
