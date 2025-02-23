use crate::server::utils::get_optional_header;
use actix_web::HttpRequest;

pub fn init_ayb_client(
    config: &crate::server::config::AybConfig,
    req: &HttpRequest,
) -> crate::client::http::AybClient {
    let mut request_token: Option<String> = None;
    // Get auth token from cookie if present
    if let Ok(Some(token)) = get_optional_header(req, "Cookie") {
        if let Some(auth_token) = token
            .split(';')
            .find(|c| c.trim().starts_with("auth="))
            .map(|c| c.trim()[5..].to_string())
        {
            request_token = Some(auth_token);
        }
    }

    crate::client::http::AybClient {
        base_url: format!("http://localhost:{}", config.port),
        api_token: request_token,
    }
}
