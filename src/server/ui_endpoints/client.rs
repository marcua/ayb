use crate::server::utils::get_optional_header;
use actix_web::HttpRequest;

pub struct AuthenticationDetails {
    pub username: String,
    pub token: String,
}

pub fn init_ayb_client(
    config: &crate::server::config::AybConfig,
    req: &HttpRequest,
) -> crate::client::http::AybClient {
    let request_token = authentication_details(req).map(|details| details.token);

    crate::client::http::AybClient {
        base_url: format!("http://localhost:{}", config.port),
        api_token: request_token,
    }
}

pub fn authentication_details(req: &HttpRequest) -> Option<AuthenticationDetails> {
    // Get auth token from cookie if present
    if let Ok(Some(token)) = get_optional_header(req, "Cookie") {
        if let Some(auth_token) = token
            .split(';')
            .find(|c| c.trim().starts_with("auth="))
            .map(|c| c.trim()[5..].to_string())
        {
            // Parse the auth token to extract username and token parts
            let parts: Vec<&str> = auth_token.split(':').collect();
            if parts.len() >= 2 {
                return Some(AuthenticationDetails {
                    username: parts[0].to_string(),
                    token: parts[1].to_string(),
                });
            }
        }
    }
    None
}
