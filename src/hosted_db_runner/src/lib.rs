use rusqlite;
use serde::{Deserialize, Serialize};
use std::string;
use std::vec::Vec;

#[derive(Serialize, Debug, Deserialize)]
pub struct QueryResult {
    pub fields: Vec<String>,
    pub rows: Vec<Vec<Option<String>>>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct AybError {
    pub message: String,
}

impl From<std::io::Error> for AybError {
    fn from(cause: std::io::Error) -> Self {
        AybError {
            message: format!("IO error: {:?}", cause),
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

impl From<std::str::Utf8Error> for AybError {
    fn from(cause: std::str::Utf8Error) -> Self {
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
