use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, BufWriter, Write};
use std::path::PathBuf;

#[derive(Serialize, Deserialize)]
pub struct ClientConfig {
    version: u64,
    pub authentication: HashMap<String, String>,
    pub default_url: Option<String>,
}

impl ClientConfig {
    fn default_config() -> ClientConfig {
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

        Ok(ClientConfig::default_config())
    }

    pub fn to_file(&self, file_path: &PathBuf) -> Result<(), std::io::Error> {
        let file = File::create(file_path)?;
        let mut writer = BufWriter::new(file);
        serde_json::to_writer(&mut writer, &self)?;
        writer.flush()?;
        Ok(())
    }
}
