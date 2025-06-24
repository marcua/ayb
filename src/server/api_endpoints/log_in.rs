use crate::ayb_db::db_interfaces::AybDb;
use crate::ayb_db::models::{AuthenticationMethodStatus, AuthenticationMethodType};
use crate::email::send_registration_email;
use crate::error::AybError;

use crate::http::structs::{AuthenticationDetails, EmptyResponse};
use crate::server::config::AybConfig;
use crate::server::tokens::encrypt_auth_token;
use crate::server::utils::get_lowercased_header;
use crate::server::web_frontend::WebFrontendDetails;
use actix_web::{post, web, HttpRequest, HttpResponse};

#[post("/v1/log_in")]
async fn log_in(
    req: HttpRequest,
    ayb_db: web::Data<Box<dyn AybDb>>,
    ayb_config: web::Data<AybConfig>,
    web_details: web::Data<Option<WebFrontendDetails>>,
) -> Result<HttpResponse, AybError> {
    let entity = get_lowercased_header(&req, "entity")?;
    let desired_entity = ayb_db.get_entity_by_slug(&entity).await;

    if let Ok(instantiated_entity) = desired_entity {
        let auth_methods = ayb_db
            .list_authentication_methods(&instantiated_entity)
            .await?;
        for method in auth_methods {
            if AuthenticationMethodType::try_from(method.method_type)?
                == AuthenticationMethodType::Email
                && AuthenticationMethodStatus::try_from(method.status)?
                    == AuthenticationMethodStatus::Verified
            {
                let token = encrypt_auth_token(
                    &AuthenticationDetails {
                        version: 1,
                        entity,
                        entity_type: instantiated_entity.entity_type,
                        email_address: method.email_address.to_owned(),
                    },
                    &ayb_config.authentication,
                )?;
                send_registration_email(
                    &ayb_config.email,
                    &method.email_address,
                    &token,
                    web_details.get_ref(),
                )
                .await?;
                return Ok(HttpResponse::Ok().json(EmptyResponse {}));
            }
        }
    }

    Err(AybError::Other {
        message: format!("No account or email authentication method for {}", entity),
    })
}
