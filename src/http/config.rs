use fernet;
use std::fs;
use std::path::PathBuf;
use toml;

use crate::error::AybError;
use crate::http::structs::{AybConfig, AybConfigAuthentication, AybConfigEmail};

pub fn config_to_toml(ayb_config: AybConfig) -> Result<String, AybError> {
    Ok(toml::to_string(&ayb_config)?)
}

pub fn default_server_config() -> AybConfig {
    AybConfig {
        host: "0.0.0.0".to_string(),
        port: 5433,
        database_url: "sqlite://ayb_data/ayb.sqlite".to_string(),
        data_path: "./ayb_data".to_string(),
        e2e_testing: None,
        authentication: AybConfigAuthentication {
            fernet_key: fernet::Fernet::generate_key(),
            token_expiration_seconds: 3600,
        },
        email: AybConfigEmail {
            from: "Server Sender <server@example.org>".to_string(),
            reply_to: "Server Reply <replyto@example.org>".to_string(),
            smtp_host: "localhost".to_string(),
            smtp_port: 465,
            smtp_username: "login@example.org".to_string(),
            smtp_password: "the_password".to_string(),
            templates: None,
        },
    }
}

pub fn read_config(config_path: &PathBuf) -> Result<AybConfig, AybError> {
    let contents = fs::read_to_string(config_path)?;
    Ok(toml::from_str(&contents)?)
}
