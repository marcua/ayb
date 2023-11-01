use crate::ayb_db::models::InstantiatedEntity;
use crate::error::AybError;
use actix_web::{HttpMessage, HttpRequest};

pub fn get_authenticated_entity(req: &HttpRequest) -> Result<InstantiatedEntity, AybError> {
    match req.extensions().get::<InstantiatedEntity>() {
        Some(entity) => Ok(entity.clone()),
        None => Err(AybError {
            message: "No authenticated entity".to_string(),
        }),
    }
}

pub fn get_header(req: &HttpRequest, header_name: &str) -> Result<String, AybError> {
    match req.headers().get(header_name) {
        Some(header) => match header.to_str() {
            Ok(header_value) => Ok(header_value.to_owned()),
            Err(err) => Err(AybError {
                message: err.to_string(),
            }),
        },
        None => Err(AybError {
            message: format!("Missing required `{}` header", header_name),
        }),
    }
}

pub fn get_lowercased_header(req: &HttpRequest, header_name: &str) -> Result<String, AybError> {
    return Ok(get_header(req, header_name)?.to_lowercase());
}
