use actix_web;
use derive_more::{Display, Error};
use reqwest;
use rusqlite;
use serde::{Deserialize, Serialize};
use sqlx;

#[derive(Debug, Deserialize, Display, Error, Serialize)]
pub struct AybError {
    pub message: String,
}

impl actix_web::error::ResponseError for AybError {
    fn error_response(&self) -> actix_web::HttpResponse {
        actix_web::HttpResponse::InternalServerError().json(self)
    }
}

impl From<rusqlite::Error> for AybError {
    fn from(cause: rusqlite::Error) -> Self {
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
