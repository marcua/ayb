use crate::ayb_db::models::InstantiatedEntity;
use crate::error::AybError;
use actix_web::{web, HttpRequest};

pub fn get_optional_header(
    req: &HttpRequest,
    header_name: &str,
) -> Result<Option<String>, AybError> {
    match req.headers().get(header_name) {
        Some(header) => match header.to_str() {
            Ok(header_value) => Ok(Some(header_value.to_string())),
            Err(err) => Err(AybError::Other {
                message: err.to_string(),
            }),
        },
        None => Ok(None),
    }
}

pub fn get_required_header(req: &HttpRequest, header_name: &str) -> Result<String, AybError> {
    let value = get_optional_header(req, header_name)?;
    match value {
        Some(value) => Ok(value),
        None => Err(AybError::Other {
            message: format!("Missing required `{header_name}` header"),
        }),
    }
}

pub fn get_lowercased_header(req: &HttpRequest, header_name: &str) -> Result<String, AybError> {
    Ok(get_required_header(req, header_name)?.to_lowercase())
}

pub fn unwrap_authenticated_entity(
    entity: &Option<web::ReqData<InstantiatedEntity>>,
) -> Result<InstantiatedEntity, AybError> {
    match entity {
        Some(instantiated_entity) => Ok(instantiated_entity.clone().into_inner()),
        None => Err(AybError::Other {
            message: "Endpoint requires an entity, but one was not provided".to_string(),
        }),
    }
}
