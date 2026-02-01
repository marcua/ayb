use crate::ayb_db::db_interfaces::AybDb;
use crate::ayb_db::models::APIToken;
use crate::error::AybError;
use crate::http::structs::{OAuthErrorResponse, OAuthTokenRequest, OAuthTokenResponse};
use crate::server::config::AybConfig;
use crate::server::web_frontend::local_base_url;
use actix_web::{post, web, HttpResponse, Result};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use chrono::Utc;
use prefixed_api_key::rand::rngs::OsRng;
use prefixed_api_key::sha2::Sha256 as PakSha256;
use prefixed_api_key::PrefixedApiKeyController;
use sha2::{Digest, Sha256};

fn verify_pkce(code_verifier: &str, code_challenge: &str) -> bool {
    // Compute SHA256 hash of the verifier
    let mut hasher = Sha256::new();
    hasher.update(code_verifier.as_bytes());
    let hash = hasher.finalize();

    // Base64 URL encode the hash
    let computed_challenge = URL_SAFE_NO_PAD.encode(hash);

    // Use constant-time comparison to prevent timing attacks
    constant_time_eq(computed_challenge.as_bytes(), code_challenge.as_bytes())
}

fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut result = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        result |= x ^ y;
    }
    result == 0
}

fn i16_to_permission_level(level: i16) -> &'static str {
    match level {
        0 => "read-only",
        1 => "read-write",
        _ => "unknown",
    }
}

#[post("/v1/oauth/token")]
pub async fn oauth_token(
    body: web::Json<OAuthTokenRequest>,
    ayb_db: web::Data<Box<dyn AybDb>>,
    ayb_config: web::Data<AybConfig>,
) -> Result<HttpResponse> {
    // Validate grant_type
    if body.grant_type != "authorization_code" {
        return Ok(HttpResponse::BadRequest().json(OAuthErrorResponse {
            error: "unsupported_grant_type".to_string(),
            error_description: Some("Only grant_type=authorization_code is supported".to_string()),
        }));
    }

    // Look up the authorization request
    let auth_request = match ayb_db.get_oauth_authorization_request(&body.code).await {
        Ok(req) => req,
        Err(AybError::RecordNotFound { .. }) => {
            return Ok(HttpResponse::BadRequest().json(OAuthErrorResponse {
                error: "invalid_grant".to_string(),
                error_description: Some("Authorization code not found".to_string()),
            }));
        }
        Err(err) => {
            return Ok(
                HttpResponse::InternalServerError().json(OAuthErrorResponse {
                    error: "server_error".to_string(),
                    error_description: Some(err.to_string()),
                }),
            );
        }
    };

    // Check if code has already been used
    if auth_request.used_at.is_some() {
        return Ok(HttpResponse::BadRequest().json(OAuthErrorResponse {
            error: "invalid_grant".to_string(),
            error_description: Some("Authorization code has already been used".to_string()),
        }));
    }

    // Check if code has expired
    if auth_request.expires_at < Utc::now().naive_utc() {
        return Ok(HttpResponse::BadRequest().json(OAuthErrorResponse {
            error: "invalid_grant".to_string(),
            error_description: Some("Authorization code has expired".to_string()),
        }));
    }

    // Verify redirect_uri matches
    if auth_request.redirect_uri != body.redirect_uri {
        return Ok(HttpResponse::BadRequest().json(OAuthErrorResponse {
            error: "invalid_grant".to_string(),
            error_description: Some("redirect_uri does not match".to_string()),
        }));
    }

    // Verify PKCE
    if !verify_pkce(&body.code_verifier, &auth_request.code_challenge) {
        return Ok(HttpResponse::BadRequest().json(OAuthErrorResponse {
            error: "invalid_grant".to_string(),
            error_description: Some("PKCE verification failed".to_string()),
        }));
    }

    // Mark the authorization code as used
    if let Err(err) = ayb_db
        .mark_oauth_authorization_request_used(&body.code)
        .await
    {
        return Ok(
            HttpResponse::InternalServerError().json(OAuthErrorResponse {
                error: "server_error".to_string(),
                error_description: Some(err.to_string()),
            }),
        );
    }

    // Get the database and entity info for the response
    let database = match ayb_db.get_database_by_id(auth_request.database_id).await {
        Ok(db) => db,
        Err(err) => {
            return Ok(
                HttpResponse::InternalServerError().json(OAuthErrorResponse {
                    error: "server_error".to_string(),
                    error_description: Some(err.to_string()),
                }),
            );
        }
    };

    let entity = match ayb_db.get_entity_by_id(database.entity_id).await {
        Ok(e) => e,
        Err(err) => {
            return Ok(
                HttpResponse::InternalServerError().json(OAuthErrorResponse {
                    error: "server_error".to_string(),
                    error_description: Some(err.to_string()),
                }),
            );
        }
    };

    // Create a new scoped API token
    let controller: PrefixedApiKeyController<OsRng, PakSha256> =
        PrefixedApiKeyController::configure()
            .prefix("ayb".to_owned())
            .seam_defaults()
            .finalize()
            .map_err(AybError::from)?;

    let (pak, hash) = controller.generate_key_and_hash();
    let token_string = pak.to_string();
    let short_token = pak.short_token().to_string();

    let api_token = APIToken {
        entity_id: auth_request.entity_id,
        short_token: short_token.clone(),
        hash,
        database_id: Some(auth_request.database_id),
        query_permission_level: Some(auth_request.query_permission_level),
        app_name: Some(auth_request.app_name.clone()),
        created_at: Some(Utc::now().naive_utc()),
        expires_at: None, // OAuth tokens don't expire by default
        revoked_at: None,
    };

    if let Err(err) = ayb_db.create_api_token(&api_token).await {
        return Ok(
            HttpResponse::InternalServerError().json(OAuthErrorResponse {
                error: "server_error".to_string(),
                error_description: Some(err.to_string()),
            }),
        );
    }

    // Build the response
    let base_url = local_base_url(&ayb_config);
    let database_path = format!("{}/{}", entity.slug, database.slug);
    let database_url = format!("{base_url}/v1/{database_path}");

    Ok(HttpResponse::Ok().json(OAuthTokenResponse {
        access_token: token_string,
        token_type: "Bearer".to_string(),
        database: database_path,
        query_permission_level: i16_to_permission_level(auth_request.query_permission_level)
            .to_string(),
        database_url,
    }))
}
