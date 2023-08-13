use crate::error::AybError;
use crate::http::structs::{AuthenticationDetails, AybConfigAuthentication};
use fernet::Fernet;
use serde_json;

fn get_fernet_generator(auth_config: &AybConfigAuthentication) -> Result<Fernet, AybError> {
    match Fernet::new(&auth_config.fernet_key) {
        Some(token_generator) => Ok(token_generator),
        None => Err(AybError {
            message: "Missing or invalid Fernet key".to_string(),
        }),
    }
}

pub fn encrypt_auth_token(
    authentication_details: &AuthenticationDetails,
    auth_config: &AybConfigAuthentication,
) -> Result<String, AybError> {
    // TODO(marcua): Add `ayb server show_config` and `ayb server
    // create_config` to make setting up keys easier.
    // println!("key: {}", fernet::Fernet::generate_key());
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
