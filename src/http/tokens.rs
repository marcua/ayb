use crate::error::AybError;
use crate::http::structs::{AuthenticationDetails, AybConfigAuthentication};
use fernet::Fernet;
use serde_json;

pub fn create_token(
    authentication_details: &AuthenticationDetails,
    auth_config: &AybConfigAuthentication,
) -> Result<String, AybError> {
    // println!("key: {}", fernet::Fernet::generate_key());
    println!("key: {}", auth_config.fernet_key);
    // TODO(marcua): Add `ayb server show_config` and `ayb server
    // create_config` to make setting up keys easier.
    match Fernet::new(&auth_config.fernet_key) {
        Some(token_generator) => {
            Ok(token_generator.encrypt(&serde_json::to_vec(&authentication_details)?))
        }
        None => Err(AybError {
            message: "Missing or invalid Fernet key".to_string(),
        }),
    }
}
