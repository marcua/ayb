use crate::http::structs::APIToken;
use crate::server::utils::get_optional_header;
use crate::server::web_frontend::local_base_url;
use actix_web::{http::header, HttpRequest, HttpResponse};

pub const COOKIE_FOR_LOGOUT: &str = "auth=; Path=/; HttpOnly; Secure; SameSite=Strict; Max-Age=0";

pub fn authentication_details(req: &HttpRequest) -> Option<APIToken> {
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
                return Some(APIToken {
                    entity: parts[0].to_string(),
                    token: parts[1].to_string(),
                });
            }
        }
    }
    None
}

pub fn cookie_for_token(token: &APIToken) -> String {
    format!(
        "auth={}:{}; Path=/; HttpOnly; Secure; SameSite=Strict",
        token.entity, token.token
    )
}

pub fn redirect_to_login(req: &HttpRequest) -> HttpResponse {
    let current_url = req.uri().to_string();
    let login_url = format!("/log_in?next={}", urlencoding::encode(&current_url));
    HttpResponse::Found()
        .insert_header((header::LOCATION, login_url))
        .finish()
}

pub fn init_ayb_client(
    config: &crate::server::config::AybConfig,
    req: &HttpRequest,
) -> crate::client::http::AybClient {
    let request_token = authentication_details(req).map(|details| details.token);

    crate::client::http::AybClient {
        base_url: local_base_url(config),
        api_token: request_token,
    }
}
