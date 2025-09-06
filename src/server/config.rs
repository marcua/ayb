use fernet;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use toml;
use url::Url;

use crate::error::AybError;

#[derive(Clone, Serialize, Debug, Deserialize, PartialEq)]
pub enum WebHostingMethod {
    Local,
    Remote,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct AybConfigWeb {
    pub hosting_method: WebHostingMethod,
    pub base_url: Option<Url>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct AybConfigCors {
    pub origin: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct AybConfigAuthentication {
    pub fernet_key: String,
    pub token_expiration_seconds: u64,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct AybConfigEmailSmtp {
    pub from: String,
    pub reply_to: String,
    pub smtp_host: String,
    pub smtp_port: u16,
    pub smtp_username: String,
    pub smtp_password: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct AybConfigEmailFile {
    pub path: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct AybConfigEmailBackends {
    pub smtp: Option<AybConfigEmailSmtp>,
    pub file: Option<AybConfigEmailFile>,
}

impl AybConfigEmailBackends {
    pub fn validate(&self) -> Result<(), AybError> {
        if self.smtp.is_none() && self.file.is_none() {
            return Err(AybError::ConfigurationError {
                message: "At least one email backend (smtp or file) must be configured. See email configuration documentation at https://github.com/marcua/ayb#email-configuration".to_string(),
            });
        }
        Ok(())
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct AybConfigIsolation {
    pub nsjail_path: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub enum SqliteSnapshotMethod {
    Backup,
    Vacuum,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct AybConfigSnapshotsAutomation {
    pub interval: String, // A time interval in Go's time.ParseDuration format (e.g., "5m" means "every 5 minutes",
    pub max_snapshots: u16,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct AybConfigSnapshots {
    pub sqlite_method: SqliteSnapshotMethod,
    pub access_key_id: String,
    pub secret_access_key: String,
    pub bucket: String,
    pub path_prefix: String,
    pub endpoint_url: Option<String>,
    pub region: Option<String>,
    // By default, AWS (and some S3-compatible providers) include
    // bucket details in the domain/endpoint. Other tools like minio
    // include the bucket in the path. When `force_path_style` is
    // `true` (it defaults to `false`), we include the bucket
    // in the path.
    pub force_path_style: Option<bool>,
    pub automation: Option<AybConfigSnapshotsAutomation>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct AybConfig {
    pub host: String,
    pub port: u16,
    pub public_url: Option<String>,
    pub database_url: String,
    pub data_path: String,
    pub authentication: AybConfigAuthentication,
    pub email: AybConfigEmailBackends,
    pub web: Option<AybConfigWeb>,
    pub cors: AybConfigCors,
    pub isolation: Option<AybConfigIsolation>,
    pub snapshots: Option<AybConfigSnapshots>,
}

pub fn config_to_toml(ayb_config: AybConfig) -> Result<String, AybError> {
    Ok(toml::to_string(&ayb_config)?)
}

pub fn default_server_config() -> AybConfig {
    AybConfig {
        host: "0.0.0.0".to_string(),
        port: 5433,
        public_url: None,
        database_url: "sqlite://ayb_data/ayb.sqlite".to_string(),
        data_path: "./ayb_data".to_string(),
        authentication: AybConfigAuthentication {
            fernet_key: fernet::Fernet::generate_key(),
            token_expiration_seconds: 3600,
        },
        email: AybConfigEmailBackends {
            smtp: Some(AybConfigEmailSmtp {
                from: "Server Sender <server@example.org>".to_string(),
                reply_to: "Server Reply <replyto@example.org>".to_string(),
                smtp_host: "localhost".to_string(),
                smtp_port: 465,
                smtp_username: "login@example.org".to_string(),
                smtp_password: "the_password".to_string(),
            }),
            file: Some(AybConfigEmailFile {
                path: "./ayb_data/emails.jsonl".to_string(),
            }),
        },
        cors: AybConfigCors {
            origin: "*".to_string(),
        },
        web: Some(AybConfigWeb {
            hosting_method: WebHostingMethod::Local,
            base_url: None,
        }),
        isolation: None,
        snapshots: None,
    }
}

pub fn read_config(config_path: &PathBuf) -> Result<AybConfig, AybError> {
    let contents = fs::read_to_string(config_path).map_err(|err| AybError::ConfigurationError {
        message: err.to_string(),
    })?;
    let config: AybConfig =
        toml::from_str(&contents).map_err(|err| AybError::ConfigurationError {
            message: err.to_string(),
        })?;

    // Validate email configuration
    config.email.validate()?;

    Ok(config)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_email_backends_validation_both_configured() {
        let config = AybConfigEmailBackends {
            smtp: Some(AybConfigEmailSmtp {
                from: "test@example.com".to_string(),
                reply_to: "test@example.com".to_string(),
                smtp_host: "localhost".to_string(),
                smtp_port: 587,
                smtp_username: "test".to_string(),
                smtp_password: "test".to_string(),
            }),
            file: Some(AybConfigEmailFile {
                path: "/tmp/test.jsonl".to_string(),
            }),
        };
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_email_backends_validation_smtp_only() {
        let config = AybConfigEmailBackends {
            smtp: Some(AybConfigEmailSmtp {
                from: "test@example.com".to_string(),
                reply_to: "test@example.com".to_string(),
                smtp_host: "localhost".to_string(),
                smtp_port: 587,
                smtp_username: "test".to_string(),
                smtp_password: "test".to_string(),
            }),
            file: None,
        };
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_email_backends_validation_file_only() {
        let config = AybConfigEmailBackends {
            smtp: None,
            file: Some(AybConfigEmailFile {
                path: "/tmp/test.jsonl".to_string(),
            }),
        };
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_email_backends_validation_none_configured() {
        let config = AybConfigEmailBackends {
            smtp: None,
            file: None,
        };
        let result = config.validate();
        assert!(result.is_err());
        let error_message = result.unwrap_err().to_string();
        assert!(error_message.contains("At least one email backend"));
        assert!(error_message.contains("https://github.com/marcua/ayb#email-configuration"));
    }
}
