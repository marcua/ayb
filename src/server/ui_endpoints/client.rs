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
            // TODO(marcua): Why does this work? Don't we need to split the token and pass only the second part?
            request_token = Some(auth_token);
        }
    }

    crate::client::http::AybClient {
        base_url: format!("http://localhost:{}", config.port),
        api_token: request_token,
    }
}

pub fn logged_in_entity(req: &HttpRequest) -> Option<String> {
    // Get auth token from cookie if present
    if let Ok(Some(token)) = get_optional_header(req, "Cookie") {
        if let Some(auth_token) = token
            .split(';')
            .find(|c| c.trim().starts_with("auth="))
            .map(|c| c.trim()[5..].to_string())
        {
            // If we have an auth token, we need to extract the entity slug
            // The entity slug should be stored in the cookie when confirming login
            if let Some(entity_part) = auth_token.split(':').nth(0) {
                return Some(entity_part.to_string());
            }
        }
    }
    None
}
