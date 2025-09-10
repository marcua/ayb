use crate::ayb_db::models::InstantiatedEntity;
use crate::error::AybError;
use actix_web::{web, HttpRequest};
use std::path::PathBuf;

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

/// Extract the file path from a SQLite database URL
///
/// Converts URLs like "sqlite://path/to/database.sqlite" to "path/to/database.sqlite"
/// Supports both relative and absolute paths.
pub fn extract_sqlite_file_path(database_url: &str) -> Result<PathBuf, AybError> {
    if !database_url.starts_with("sqlite://") {
        return Err(AybError::Other {
            message: format!("Database URL '{database_url}' is not a SQLite URL"),
        });
    }

    let path_str = database_url.strip_prefix("sqlite://").unwrap();
    if path_str.is_empty() {
        return Err(AybError::Other {
            message: "SQLite database URL is missing file path".to_string(),
        });
    }

    Ok(PathBuf::from(path_str))
}

/// Check if a database URL is for SQLite
pub fn is_sqlite_database_url(database_url: &str) -> bool {
    database_url.starts_with("sqlite://")
}
