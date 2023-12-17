use actix_web;
use derive_more::{Display, Error};
use fernet;
use lettre;
use prefixed_api_key;
use quoted_printable;
use reqwest;
use rusqlite;
use serde::{Deserialize, Serialize};
use serde_json;
use sqlx;
use std::string;
use toml;

#[derive(Debug, Deserialize, Display, Error, Serialize)]
#[serde(tag = "type", content = "value")]
pub enum AybError {
    RecordNotFound,
    Other { message: String },
}

impl actix_web::error::ResponseError for AybError {
    fn error_response(&self) -> actix_web::HttpResponse {
        actix_web::HttpResponse::InternalServerError().json(self)
    }
}

impl From<fernet::DecryptionError> for AybError {
    fn from(_cause: fernet::DecryptionError) -> Self {
        AybError::Other {
            message: "Invalid or expired token".to_owned(),
        }
    }
}

impl From<lettre::address::AddressError> for AybError {
    fn from(cause: lettre::address::AddressError) -> Self {
        AybError::Other {
            message: format!("Invalid email address: {}", cause),
        }
    }
}

impl From<prefixed_api_key::BuilderError> for AybError {
    fn from(cause: prefixed_api_key::BuilderError) -> Self {
        AybError::Other {
            message: format!("Error in prefixed API key builder: {}", cause),
        }
    }
}

impl From<prefixed_api_key::PrefixedApiKeyError> for AybError {
    fn from(cause: prefixed_api_key::PrefixedApiKeyError) -> Self {
        AybError::Other {
            message: format!("Error parsing API token: {}", cause),
        }
    }
}

impl From<quoted_printable::QuotedPrintableError> for AybError {
    fn from(cause: quoted_printable::QuotedPrintableError) -> Self {
        AybError::Other {
            message: format!("{:?}", cause),
        }
    }
}

impl From<rusqlite::Error> for AybError {
    fn from(cause: rusqlite::Error) -> Self {
        AybError::Other {
            message: format!("{:?}", cause),
        }
    }
}

impl From<rusqlite::types::FromSqlError> for AybError {
    fn from(cause: rusqlite::types::FromSqlError) -> Self {
        AybError::Other {
            message: format!("{:?}", cause),
        }
    }
}

impl From<string::FromUtf8Error> for AybError {
    fn from(cause: string::FromUtf8Error) -> Self {
        AybError::Other {
            message: format!("{:?}", cause),
        }
    }
}

impl From<serde_json::Error> for AybError {
    fn from(cause: serde_json::Error) -> Self {
        AybError::Other {
            message: format!("{:?}", cause),
        }
    }
}

impl From<std::str::Utf8Error> for AybError {
    fn from(cause: std::str::Utf8Error) -> Self {
        AybError::Other {
            message: format!("{:?}", cause),
        }
    }
}

impl From<std::io::Error> for AybError {
    fn from(cause: std::io::Error) -> Self {
        AybError::Other {
            message: format!("IO error: {:?}", cause),
        }
    }
}

impl From<sqlx::Error> for AybError {
    fn from(cause: sqlx::Error) -> Self {
        match cause {
            sqlx::Error::RowNotFound => Self::RecordNotFound,
            _ => Self::Other {
                message: format!("{:?}", cause),
            }
        }
    }
}

impl From<reqwest::Error> for AybError {
    fn from(cause: reqwest::Error) -> Self {
        AybError::Other {
            message: format!("{:?}", cause),
        }
    }
}

impl From<toml::de::Error> for AybError {
    fn from(cause: toml::de::Error) -> Self {
        AybError::Other {
            message: format!("Unable to deserialize toml string: {:?}", cause),
        }
    }
}

impl From<toml::ser::Error> for AybError {
    fn from(cause: toml::ser::Error) -> Self {
        AybError::Other {
            message: format!("Unable to serialize toml string: {:?}", cause),
        }
    }
}
