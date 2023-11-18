use crate::ayb_db::db_interfaces::AybDb;
use crate::ayb_db::models::{AuthenticationMethodType, EntityType};
use crate::email::send_registration_email;
use crate::error::AybError;
use crate::http::structs::{AuthenticationDetails, AybConfig, EmptyResponse};
use crate::http::tokens::encrypt_auth_token;
use crate::http::utils::{get_header, get_lowercased_header};
use actix_web::{post, web, HttpRequest, HttpResponse};
use std::str::FromStr;

#[post("/v1/register")]
async fn register(
    req: HttpRequest,
    ayb_db: web::Data<Box<dyn AybDb>>,
    ayb_config: web::Data<AybConfig>,
) -> Result<HttpResponse, AybError> {
    let entity = get_lowercased_header(&req, "entity")?;
    let email_address = get_lowercased_header(&req, "email-address")?;
    let entity_type = get_header(&req, "entity-type")?;
    let desired_entity = ayb_db.get_entity_by_slug(&entity).await;
    // Ensure that there are no authentication methods aside from
    // perhaps the currently requested one.
    let mut already_verified = false;
    if let Ok(instantiated_entity) = desired_entity {
        let auth_methods = ayb_db
            .list_authentication_methods(&instantiated_entity)
            .await?;
        for method in auth_methods {
            if AuthenticationMethodType::try_from(method.method_type)?
                != AuthenticationMethodType::Email
                || method.email_address != email_address
            {
                already_verified = true;
                break;
            }
        }
    }

    if already_verified {
        return Err(AybError {
            message: format!("{} has already been registered", entity),
        });
    }

    let token = encrypt_auth_token(
        &AuthenticationDetails {
            version: 1,
            entity: entity.clone(),
            entity_type: EntityType::from_str(&entity_type)? as i16,
            email_address: email_address.to_owned(),
        },
        &ayb_config.authentication,
    )?;
    send_registration_email(
        &email_address,
        &token,
        &ayb_config.email,
        ayb_config.e2e_testing_on(),
    )
    .await?;
    Ok(HttpResponse::Ok().json(EmptyResponse {}))
}
