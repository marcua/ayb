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

#[derive(Debug, Deserialize, Display, Error, Serialize)]
pub struct AybError {
    pub message: String,
}

impl actix_web::error::ResponseError for AybError {
    fn error_response(&self) -> actix_web::HttpResponse {
        actix_web::HttpResponse::InternalServerError().json(self)
    }
}

impl From<fernet::DecryptionError> for AybError {
    fn from(_cause: fernet::DecryptionError) -> Self {
        AybError {
            message: "Invalid or expired token".to_owned(),
        }
    }
}

impl From<lettre::address::AddressError> for AybError {
    fn from(cause: lettre::address::AddressError) -> Self {
        AybError {
            message: format!("Invalid email address: {}", cause),
        }
    }
}

impl From<prefixed_api_key::BuilderError> for AybError {
    fn from(cause: prefixed_api_key::BuilderError) -> Self {
        AybError {
            message: format!("Error in prefixed API key builder: {}", cause),
        }
    }
}

impl From<prefixed_api_key::PrefixedApiKeyError> for AybError {
    fn from(cause: prefixed_api_key::PrefixedApiKeyError) -> Self {
        AybError {
            message: format!("Error parsing API token: {}", cause)
        }
    }
}

impl From<quoted_printable::QuotedPrintableError> for AybError {
    fn from(cause: quoted_printable::QuotedPrintableError) -> Self {
        AybError {
            message: format!("{:?}", cause),
        }
    }
}

impl From<rusqlite::Error> for AybError {
    fn from(cause: rusqlite::Error) -> Self {
        AybError {
            message: format!("{:?}", cause),
        }
    }
}

impl From<rusqlite::types::FromSqlError> for AybError {
    fn from(cause: rusqlite::types::FromSqlError) -> Self {
        AybError {
            message: format!("{:?}", cause),
        }
    }
}

impl From<string::FromUtf8Error> for AybError {
    fn from(cause: string::FromUtf8Error) -> Self {
        AybError {
            message: format!("{:?}", cause),
        }
    }
}

impl From<serde_json::Error> for AybError {
    fn from(cause: serde_json::Error) -> Self {
        AybError {
            message: format!("{:?}", cause),
        }
    }
}

impl From<std::str::Utf8Error> for AybError {
    fn from(cause: std::str::Utf8Error) -> Self {
        AybError {
            message: format!("{:?}", cause),
        }
    }
}

impl From<sqlx::Error> for AybError {
    fn from(cause: sqlx::Error) -> Self {
        AybError {
            message: format!("{:?}", cause),
        }
    }
}

impl From<reqwest::Error> for AybError {
    fn from(cause: reqwest::Error) -> Self {
        AybError {
            message: format!("{:?}", cause),
        }
    }
}
