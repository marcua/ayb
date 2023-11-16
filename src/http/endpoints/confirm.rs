use crate::ayb_db::db_interfaces::AybDb;
use crate::ayb_db::models::{
    AuthenticationMethod, AuthenticationMethodStatus, AuthenticationMethodType,
    Entity, InstantiatedAuthenticationMethod,
};
use crate::error::AybError;
use crate::http::structs::{
    APIToken as APIAPIToken, AybConfig,
};
use crate::http::tokens::{decrypt_auth_token, generate_api_token};
use crate::http::utils::{get_header};
use actix_web::{post, web, HttpRequest, HttpResponse};

#[post("/v1/confirm")]
async fn confirm(
    req: HttpRequest,
    ayb_db: web::Data<Box<dyn AybDb>>,
    ayb_config: web::Data<AybConfig>,
) -> Result<HttpResponse, AybError> {
    let auth_token = get_header(&req, "authentication-token")?;
    let auth_details = decrypt_auth_token(auth_token, &ayb_config.authentication)?;

    let created_entity = ayb_db
        .get_or_create_entity(&Entity {
            slug: auth_details.entity,
            entity_type: auth_details.entity_type,
        })
        .await?;

    // Check if there are authentication methods already, and if this
    // method in particular is verified.
    let auth_methods = ayb_db.list_authentication_methods(&created_entity).await?;
    let mut already_verified = false;
    let mut found_auth_method: Option<InstantiatedAuthenticationMethod> = None;
    for method in auth_methods {
        already_verified = true;
        if method.method_type == (AuthenticationMethodType::Email as i16)
            && method.email_address == auth_details.email_address
        {
            found_auth_method = Some(method)
        }
    }

    if found_auth_method.is_none() {
        // If the user was logging in to an already verified account,
        // auth_method can't be empty. So the only way to reach this
        // branch is when registering.
        // When registering, either accept this authentication method
        // if it was previously created, or if there is no other
        // verification method already verified.
        if already_verified {
            return Err(AybError {
                message: format!("{} has already been registered", created_entity.slug),
            });
        }
        ayb_db
            .create_authentication_method(&AuthenticationMethod {
                entity_id: created_entity.id,
                method_type: AuthenticationMethodType::Email as i16,
                status: AuthenticationMethodStatus::Verified as i16,
                email_address: auth_details.email_address,
            })
            .await?;
    }

    let (api_token, token_string) = generate_api_token(&created_entity)?;
    let _ = ayb_db.create_api_token(&api_token).await?;
    let returned_token = APIAPIToken {
        token: token_string,
    };

    Ok(HttpResponse::Ok().json(returned_token))
}

#[cfg(test)]
mod tests {
    use actix_web::{App, test};
    use crate::http::tokens::encrypt_auth_token;
    use crate::ayb_db::{db_interfaces::connect_to_ayb_db, models::EntityType};
    use crate::http::structs::{AybConfig, AybConfigEmail, AybConfigAuthentication, AuthenticationDetails};
    use super::*;

    #[actix_web::test]
    async fn v1_confirm_post() {
        let db = connect_to_ayb_db("sqlite://:memory:".into()).await.unwrap();
        let ayb_conf = AybConfig {
            host: "0.0.0.0".into(),
            port: 5433,
            e2e_testing: Some(true),
            database_url: "sqlite://:memory:".into(),
            data_path: "./ayb_data".into(),
            authentication: AybConfigAuthentication {
                fernet_key: "QRibF1t12YQAwtCucF8RbBB_RHp9g92j1-wjxYJXiBc=".into(),
                token_expiration_seconds: 31536000,
            },
            email: AybConfigEmail {
                from: "".into(),
                reply_to: "".into(),
                smtp_host: "".into(),
                smtp_port: 0,
                smtp_username: "".into(),
                smtp_password: "".into(),
            },
        };
        let app = test::init_service(
        App::new()
                .app_data(web::Data::new(ayb_conf.clone()))
                .app_data(web::Data::new(db))
                .service(confirm)
        ).await;
        let req = test::TestRequest::post()
            .uri("/v1/confirm")
            .insert_header(("authentication-token", encrypt_auth_token(
                &AuthenticationDetails {
                    version: 1,
                    entity: "entity".into(),
                    entity_type: EntityType::User as i16,
                    email_address: "entity@localhost".into(),
                },
                &ayb_conf.authentication,
            ).unwrap()))
            .to_request();

        let _resp: APIAPIToken = test::call_and_read_body_json(&app, req).await;
    }
}