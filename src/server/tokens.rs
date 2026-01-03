use crate::ayb_db::db_interfaces::AybDb;
use crate::ayb_db::models::{APIToken, InstantiatedEntity};
use crate::error::AybError;
use crate::http::structs::AuthenticationDetails;
use crate::server::config::AybConfigAuthentication;
use actix_web::web;
use fernet::Fernet;
use prefixed_api_key::rand::rngs::OsRng;
use prefixed_api_key::sha2::Sha256;
use prefixed_api_key::{PrefixedApiKey, PrefixedApiKeyController};
use serde_json;

const API_TOKEN_PREFIX: &str = "ayb";

fn get_fernet_generator(auth_config: &AybConfigAuthentication) -> Result<Fernet, AybError> {
    match Fernet::new(&auth_config.fernet_key) {
        Some(token_generator) => Ok(token_generator),
        None => Err(AybError::Other {
            message: "Missing or invalid Fernet key".to_string(),
        }),
    }
}

pub fn encrypt_auth_token(
    authentication_details: &AuthenticationDetails,
    auth_config: &AybConfigAuthentication,
) -> Result<String, AybError> {
    let generator = get_fernet_generator(auth_config)?;
    Ok(generator.encrypt(&serde_json::to_vec(&authentication_details)?))
}

pub fn decrypt_auth_token(
    cyphertext: String,
    auth_config: &AybConfigAuthentication,
) -> Result<AuthenticationDetails, AybError> {
    let generator = get_fernet_generator(auth_config)?;
    Ok(serde_json::from_slice(&generator.decrypt_with_ttl(
        &cyphertext,
        auth_config.token_expiration_seconds,
    )?)?)
}

fn api_key_controller() -> Result<PrefixedApiKeyController<OsRng, Sha256>, AybError> {
    Ok(PrefixedApiKeyController::configure()
        .prefix(API_TOKEN_PREFIX.to_owned())
        .seam_defaults()
        .finalize()?)
}

pub fn generate_api_token(entity: &InstantiatedEntity) -> Result<(APIToken, String), AybError> {
    let controller = api_key_controller()?;
    let (pak, hash) = controller.generate_key_and_hash();
    Ok((
        APIToken {
            entity_id: entity.id,
            short_token: pak.short_token().to_string(),
            hash,
            database_id: None,
            query_permission_level: None,
            app_name: None,
            created_at: Some(chrono::Utc::now().naive_utc()),
            expires_at: None,
            revoked_at: None,
        },
        pak.to_string(),
    ))
}

pub async fn retrieve_and_validate_api_token(
    token: &str,
    ayb_db: &web::Data<Box<dyn AybDb>>,
) -> Result<APIToken, AybError> {
    let controller = api_key_controller()?;
    let pak = PrefixedApiKey::from_string(token)?;
    let api_token = (ayb_db.get_api_token(pak.short_token())).await?;
    if !controller.check_hash(&pak, &api_token.hash) {
        return Err(AybError::InvalidToken {
            message: "Invalid API token".to_string(),
        });
    }

    // Check if token is revoked (revoked_at is set)
    if api_token.revoked_at.is_some() {
        return Err(AybError::InvalidToken {
            message: "API token has been revoked".to_string(),
        });
    }

    // Check if token is expired
    if let Some(expires_at) = api_token.expires_at {
        if expires_at < chrono::Utc::now().naive_utc() {
            return Err(AybError::InvalidToken {
                message: "API token has expired".to_string(),
            });
        }
    }

    Ok(api_token)
}
