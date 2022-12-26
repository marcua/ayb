use actix_web;
use derive_more::{Display, Error};
use rusqlite;
use sqlx;

#[derive(Debug, Display, Error)]
#[display(fmt = "{}", error_string)]
pub struct StacksError {
    pub error_string: String,
}

impl actix_web::error::ResponseError for StacksError {}

impl From<rusqlite::Error> for StacksError {
    fn from(cause: rusqlite::Error) -> Self {
        StacksError {error_string: format!("{:?}", cause)}
    }
}

impl From<sqlx::Error> for StacksError {
    fn from(cause: sqlx::Error) -> Self {
        StacksError {error_string: format!("{:?}", cause)}
    }
}
