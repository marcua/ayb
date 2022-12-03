use actix_web::error;
use derive_more::{Display, Error};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct OwnerDatabase {
    pub owner: String,
    pub database: String,
}

#[derive(Serialize, Deserialize)]
pub struct Owner {
    pub owner: String,
}

#[derive(Debug, Display, Error)]
#[display(fmt = "{}", error_string)]
pub struct Error {
    pub error_string: String,
}

impl error::ResponseError for Error {}
