use crate::error::AybError;
use actix_web::HttpRequest;

pub fn get_header(req: HttpRequest, header_name: &str) -> Result<String, AybError> {
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
