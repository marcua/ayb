use crate::ayb_db::db_interfaces::AybDb;
use crate::error::AybError;
use crate::hosted_db::QueryMode;
use crate::http::structs::{OAuthErrorResponse, OAuthTokenRequest, OAuthTokenResponse};
use crate::server::config::AybConfig;
use crate::server::tokens::generate_scoped_api_token;
use crate::server::web_frontend::public_base_url;
use actix_web::{post, web, HttpResponse, Result};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use chrono::Utc;
use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;

fn verify_pkce(code_verifier: &str, code_challenge: &str) -> bool {
    // Compute SHA256 hash of the verifier
    let mut hasher = Sha256::new();
    hasher.update(code_verifier.as_bytes());
    let hash = hasher.finalize();

    // Base64 URL encode the hash
    let computed_challenge = URL_SAFE_NO_PAD.encode(hash);

    // Use constant-time comparison to prevent timing attacks
    computed_challenge
        .as_bytes()
        .ct_eq(code_challenge.as_bytes())
        .into()
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

    // Create a new scoped API token
    let (api_token, token_string) = match generate_scoped_api_token(
        auth_request.entity_id,
        auth_request.database_id,
        auth_request.query_permission_level,
        auth_request.app_name.clone(),
    ) {
        Ok(result) => result,
        Err(err) => {
            return Ok(
                HttpResponse::InternalServerError().json(OAuthErrorResponse {
                    error: "server_error".to_string(),
                    error_description: Some(err.to_string()),
                }),
            );
        }
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
    let base_url = public_base_url(&ayb_config);
    let database_path = format!(
        "{}/{}",
        auth_request.entity_slug, auth_request.database_slug
    );
    let database_url = format!("{base_url}/v1/{database_path}");

    let permission_str =
        QueryMode::try_from(auth_request.query_permission_level).map(|q| q.to_str().to_string())?;

    Ok(HttpResponse::Ok().json(OAuthTokenResponse {
        access_token: token_string,
        token_type: "Bearer".to_string(),
        database: database_path,
        query_permission_level: permission_str,
        database_url,
    }))
}
