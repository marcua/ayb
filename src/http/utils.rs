use crate::ayb_db::models::InstantiatedEntity;
use crate::error::AybError;
use actix_web::{web, HttpRequest};

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

pub fn unwrap_authenticated_entity(
    entity: &Option<web::ReqData<InstantiatedEntity>>,
) -> Result<InstantiatedEntity, AybError> {
    return match entity {
        Some(instantiated_entity) => Ok(instantiated_entity.clone().into_inner()),
        None => Err(AybError {
            message: "Endpoint requires an entity, but one was not provided".to_string(),
        }),
    };
}
