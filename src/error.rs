use actix_web;
use derive_more::{Display, Error};
use reqwest;
use rusqlite;
use serde::{Deserialize, Serialize};
use sqlx;

#[derive(Debug, Deserialize, Display, Error, Serialize)]
pub struct StacksError {
    pub message: String,
}

impl actix_web::error::ResponseError for StacksError {
    fn error_response(&self) -> actix_web::HttpResponse {
        actix_web::HttpResponse::InternalServerError().json(self)
    }
}

impl From<rusqlite::Error> for StacksError {
    fn from(cause: rusqlite::Error) -> Self {
        StacksError {
            message: format!("{:?}", cause),
        }
    }
}

impl From<sqlx::Error> for StacksError {
    fn from(cause: sqlx::Error) -> Self {
        StacksError {
            message: format!("{:?}", cause),
        }
    }
}

impl From<reqwest::Error> for StacksError {
    fn from(cause: reqwest::Error) -> Self {
        StacksError {
            message: format!("{:?}", cause),
        }
    }
}
