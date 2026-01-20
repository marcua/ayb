use crate::ayb_db::db_interfaces::AybDb;
use crate::ayb_db::models::NewOAuthAuthorizationRequest;
use crate::http::structs::{OAuthAuthorizeRequest, OAuthAuthorizeSubmit};
use crate::server::config::AybConfig;
use crate::server::ui_endpoints::auth::{authentication_details, init_ayb_client};
use crate::server::ui_endpoints::templates::{ok_response, render};
use actix_web::{get, http::header, post, web, HttpRequest, HttpResponse, Result};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use prefixed_api_key::rand::rngs::OsRng;
use prefixed_api_key::rand::RngCore;

fn generate_authorization_code() -> String {
    let mut bytes = [0u8; 32];
    OsRng.fill_bytes(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}

fn validate_redirect_uri(uri: &str) -> bool {
    // Allow https:// or http://localhost for development
    if uri.starts_with("https://") {
        return true;
    }
    if uri.starts_with("http://localhost") || uri.starts_with("http://127.0.0.1") {
        return true;
    }
    false
}

fn permission_level_to_i16(level: &str) -> Option<i16> {
    match level {
        "read-only" => Some(0),  // QueryMode::ReadOnly
        "read-write" => Some(1), // QueryMode::ReadWrite
        _ => None,
    }
}

#[get("/oauth/authorize")]
pub async fn oauth_authorize(
    req: HttpRequest,
    query: web::Query<OAuthAuthorizeRequest>,
    ayb_config: web::Data<AybConfig>,
) -> Result<HttpResponse> {
    let logged_in_entity = authentication_details(&req).map(|details| details.entity);

    // Validate required parameters
    if query.response_type != "code" {
        return Ok(oauth_error_page(
            "unsupported_response_type",
            "Only response_type=code is supported",
        ));
    }

    if query.code_challenge_method != "S256" {
        return Ok(oauth_error_page(
            "invalid_request",
            "Only code_challenge_method=S256 is supported",
        ));
    }

    if !validate_redirect_uri(&query.redirect_uri) {
        return Ok(oauth_error_page(
            "invalid_request",
            "redirect_uri must use https:// (or http://localhost for development)",
        ));
    }

    let requested_permission = permission_level_to_i16(&query.scope);
    if requested_permission.is_none() {
        return Ok(oauth_error_page(
            "invalid_scope",
            "scope must be 'read-only' or 'read-write'",
        ));
    }

    // If not logged in, redirect to login with return URL
    if logged_in_entity.is_none() {
        let current_url = req.uri().to_string();
        let login_url = format!("/log_in?next={}", urlencoding::encode(&current_url));
        return Ok(HttpResponse::Found()
            .insert_header((header::LOCATION, login_url))
            .finish());
    }

    let entity_slug = logged_in_entity.as_ref().unwrap();

    // Get user's databases
    let client = init_ayb_client(&ayb_config, &req);
    let databases = match client.entity_details(entity_slug).await {
        Ok(entity_response) => entity_response.databases,
        Err(err) => {
            return Ok(oauth_error_page("server_error", &err.to_string()));
        }
    };

    let mut context = tera::Context::new();
    context.insert("logged_in_entity", &logged_in_entity);
    context.insert("entity", entity_slug);
    context.insert("app_name", &query.app_name);
    context.insert("requested_scope", &query.scope);
    context.insert("redirect_uri", &query.redirect_uri);
    context.insert("state", &query.state);
    context.insert("code_challenge", &query.code_challenge);
    context.insert("databases", &databases);

    ok_response("oauth_authorize.html", &context)
}

#[post("/oauth/authorize")]
pub async fn oauth_authorize_submit(
    req: HttpRequest,
    form: web::Form<OAuthAuthorizeSubmit>,
    ayb_db: web::Data<Box<dyn AybDb>>,
    _ayb_config: web::Data<AybConfig>,
) -> Result<HttpResponse> {
    let logged_in = authentication_details(&req);

    if logged_in.is_none() {
        return Ok(HttpResponse::Unauthorized().body("Not logged in"));
    }

    let auth_details = logged_in.unwrap();

    // Handle deny action
    if form.action == "deny" {
        let redirect_url = build_redirect_url(
            &form.redirect_uri,
            form.state.as_deref(),
            None,
            Some("access_denied"),
        );
        return Ok(HttpResponse::Found()
            .insert_header((header::LOCATION, redirect_url))
            .finish());
    }

    // Validate permission level
    let permission_level = match permission_level_to_i16(&form.permission_level) {
        Some(level) => level,
        None => {
            return Ok(oauth_error_page(
                "invalid_request",
                "Invalid permission level",
            ));
        }
    };

    let requested_permission = match permission_level_to_i16(&form.requested_scope) {
        Some(level) => level,
        None => {
            return Ok(oauth_error_page(
                "invalid_request",
                "Invalid requested scope",
            ));
        }
    };

    // User cannot grant more permission than requested
    if permission_level > requested_permission {
        return Ok(oauth_error_page(
            "invalid_request",
            "Cannot grant more permission than requested",
        ));
    }

    // Get entity ID
    let entity = match ayb_db.get_entity_by_slug(&auth_details.entity).await {
        Ok(e) => e,
        Err(err) => {
            return Ok(oauth_error_page("server_error", &err.to_string()));
        }
    };

    // Parse database path (entity/slug format)
    let db_parts: Vec<&str> = form.database.splitn(2, '/').collect();
    if db_parts.len() != 2 {
        return Ok(oauth_error_page(
            "invalid_request",
            "Invalid database format",
        ));
    }
    let (db_entity, db_slug) = (db_parts[0], db_parts[1]);

    // Verify user owns/has access to the database
    let database = match ayb_db.get_database(db_entity, db_slug).await {
        Ok(db) => db,
        Err(err) => {
            return Ok(oauth_error_page("server_error", &err.to_string()));
        }
    };

    // Generate authorization code
    let code = generate_authorization_code();
    let expires_at = chrono::Utc::now().naive_utc() + chrono::Duration::minutes(10);

    // Store authorization request
    let auth_request = NewOAuthAuthorizationRequest {
        code: code.clone(),
        entity_id: entity.id,
        code_challenge: form.code_challenge.clone(),
        redirect_uri: form.redirect_uri.clone(),
        app_name: form.app_name.clone(),
        requested_query_permission_level: requested_permission,
        state: form.state.clone(),
        database_id: database.id,
        query_permission_level: permission_level,
        expires_at,
    };

    if let Err(err) = ayb_db
        .create_oauth_authorization_request(&auth_request)
        .await
    {
        return Ok(oauth_error_page("server_error", &err.to_string()));
    }

    // Redirect back to app with code
    let redirect_url =
        build_redirect_url(&form.redirect_uri, form.state.as_deref(), Some(&code), None);

    Ok(HttpResponse::Found()
        .insert_header((header::LOCATION, redirect_url))
        .finish())
}

fn build_redirect_url(
    base_uri: &str,
    state: Option<&str>,
    code: Option<&str>,
    error: Option<&str>,
) -> String {
    let mut url = base_uri.to_string();
    let separator = if url.contains('?') { '&' } else { '?' };

    if let Some(c) = code {
        url.push_str(&format!("{}code={}", separator, urlencoding::encode(c)));
        if let Some(s) = state {
            url.push_str(&format!("&state={}", urlencoding::encode(s)));
        }
    } else if let Some(e) = error {
        url.push_str(&format!("{}error={}", separator, urlencoding::encode(e)));
        if let Some(s) = state {
            url.push_str(&format!("&state={}", urlencoding::encode(s)));
        }
    }

    url
}

fn oauth_error_page(error: &str, description: &str) -> HttpResponse {
    let mut context = tera::Context::new();
    context.insert("error", error);
    context.insert("error_description", description);
    HttpResponse::BadRequest()
        .content_type("text/html; charset=utf-8")
        .body(render("oauth_error.html", &context))
}
