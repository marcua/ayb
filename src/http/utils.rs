use crate::http::structs::Error;
use actix_web::HttpRequest;

pub fn get_header(req: HttpRequest, header_name: &str) -> Result<String, Error> {
    match req.headers().get(header_name) {
        Some(header) => match header.to_str() {
            Ok(header_value) => Ok(header_value.to_owned()),
            Err(err) => Err(Error {
                error_string: err.to_string(),
            }),
        },
        None => Err(Error {
            error_string: format!("Missing required `{}` header", header_name),
        }),
    }
}
