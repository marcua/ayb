use actix_web;
use aws_smithy_types_convert;
use derive_more::Error;
use fernet;
use lettre;
use prefixed_api_key;
use quoted_printable;
use reqwest;
use rusqlite;
use serde::{Deserialize, Serialize};
use serde_json;
use sqlx;
use std::fmt::{Display, Formatter};
use std::string;
use toml;
use url;

#[derive(Debug, Deserialize, Error, Serialize)]
#[serde(tag = "type")]
pub enum AybError {
    DurationParseError { message: String },
    S3ExecutionError { message: String },
    S3ConnectionError { message: String },
    SnapshotError { message: String },
    RecordNotFound { id: String, record_type: String },
    Other { message: String },
}

impl Display for AybError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            AybError::Other { message } => write!(f, "{}", message),
            _ => write!(f, "{:?}", self),
        }
    }
}

impl actix_web::error::ResponseError for AybError {
    fn error_response(&self) -> actix_web::HttpResponse {
        actix_web::HttpResponse::InternalServerError().json(self)
    }
}

impl From<aws_smithy_types_convert::date_time::Error> for AybError {
    fn from(cause: aws_smithy_types_convert::date_time::Error) -> Self {
        AybError::S3ExecutionError {
            message: format!("Unable to convert from AWS datetime: {:?}", cause),
        }
    }
}

impl From<fernet::DecryptionError> for AybError {
    fn from(_cause: fernet::DecryptionError) -> Self {
        AybError::Other {
            message: "Invalid or expired token".to_owned(),
        }
    }
}

impl From<go_parse_duration::Error> for AybError {
    fn from(cause: go_parse_duration::Error) -> Self {
        AybError::DurationParseError {
            message: format!("Unable to parse duration: {:?}", cause),
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
        AybError::Other {
            message: format!("{:?}", cause),
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

impl From<tokio_cron_scheduler::JobSchedulerError> for AybError {
    fn from(cause: tokio_cron_scheduler::JobSchedulerError) -> Self {
        AybError::SnapshotError {
            message: format!("Unable to schedule snapshots: {:?}", cause),
        }
    }
}

impl From<url::ParseError> for AybError {
    fn from(cause: url::ParseError) -> Self {
        AybError::Other {
            message: format!("Failed to parse URL: {:?}", cause),
        }
    }
}
