use crate::error::StacksError;
use actix_web::HttpRequest;

pub fn get_header(req: HttpRequest, header_name: &str) -> Result<String, StacksError> {
    match req.headers().get(header_name) {
        Some(header) => match header.to_str() {
            Ok(header_value) => Ok(header_value.to_owned()),
            Err(err) => Err(StacksError {
                error_string: err.to_string(),
            }),
        },
        None => Err(StacksError {
            error_string: format!("Missing required `{}` header", header_name),
        }),
    }
}
