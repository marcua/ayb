use crate::ayb_db::db_interfaces::AybDb;
use crate::ayb_db::models::{
    AuthenticationMethod, AuthenticationMethodStatus, AuthenticationMethodType, DBType, Database,
    Entity, EntityType, InstantiatedAuthenticationMethod, InstantiatedEntity,
};
use crate::email::send_registration_email;
use crate::error::AybError;
use crate::hosted_db::paths::database_path;
use crate::hosted_db::{run_query, QueryResult};
use crate::http::permissions::{can_create_database, can_query};
use crate::http::structs::{
    APIToken as APIAPIToken, AuthenticationDetails, AybConfig, Database as APIDatabase,
    EmptyResponse, EntityDatabasePath,
};
use crate::http::tokens::{decrypt_auth_token, encrypt_auth_token, generate_api_token};
use crate::http::utils::{get_header, get_lowercased_header, unwrap_authenticated_entity};
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

    if let None = found_auth_method {
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

#[post("/v1/{entity}/{database}/create")]
async fn create_database(
    path: web::Path<EntityDatabasePath>,
    req: HttpRequest,
    ayb_db: web::Data<Box<dyn AybDb>>,
    authenticated_entity: Option<web::ReqData<InstantiatedEntity>>,
) -> Result<HttpResponse, AybError> {
    let entity_slug = &path.entity;
    let entity = ayb_db.get_entity_by_slug(entity_slug).await?;
    let db_type = get_header(&req, "db-type")?;
    let database = Database {
        entity_id: entity.id,
        slug: path.database.clone(),
        db_type: DBType::from_str(&db_type) as i16,
    };
    let authenticated_entity = unwrap_authenticated_entity(&authenticated_entity)?;
    if can_create_database(&authenticated_entity, &entity) {
        let created_database = ayb_db.create_database(&database).await?;
        Ok(HttpResponse::Created().json(APIDatabase::from_persisted(&entity, &created_database)))
    } else {
        Err(AybError {
            message: format!(
                "Authenticated entity {} can not create a database for entity {}",
                authenticated_entity.slug, entity_slug
            ),
        })
    }
}

#[post("/v1/log_in")]
async fn log_in(
    req: HttpRequest,
    ayb_db: web::Data<Box<dyn AybDb>>,
    ayb_config: web::Data<AybConfig>,
) -> Result<HttpResponse, AybError> {
    let entity = get_lowercased_header(&req, "entity")?;
    let desired_entity = ayb_db.get_entity_by_slug(&entity).await;

    if let Ok(instantiated_entity) = desired_entity {
        let auth_methods = ayb_db
            .list_authentication_methods(&instantiated_entity)
            .await?;
        for method in auth_methods {
            if AuthenticationMethodType::from_i16(method.method_type)
                == AuthenticationMethodType::Email
                && AuthenticationMethodStatus::from_i16(method.status)
                    == AuthenticationMethodStatus::Verified
            {
                let token = encrypt_auth_token(
                    &AuthenticationDetails {
                        version: 1,
                        entity: entity,
                        entity_type: instantiated_entity.entity_type,
                        email_address: method.email_address.to_owned(),
                    },
                    &ayb_config.authentication,
                )?;
                send_registration_email(
                    &method.email_address,
                    &token,
                    &ayb_config.email,
                    ayb_config.e2e_testing_on(),
                )
                .await?;
                return Ok(HttpResponse::Ok().json(EmptyResponse {}));
            }
        }
    }

    return Err(AybError {
        message: format!("No account or email authentication method for {}", entity),
    });
}

#[post("/v1/{entity}/{database}/query")]
async fn query(
    path: web::Path<EntityDatabasePath>,
    query: String,
    ayb_db: web::Data<Box<dyn AybDb>>,
    ayb_config: web::Data<AybConfig>,
    authenticated_entity: Option<web::ReqData<InstantiatedEntity>>,
) -> Result<web::Json<QueryResult>, AybError> {
    let entity_slug = &path.entity;
    let database_slug = &path.database;
    let database = ayb_db.get_database(entity_slug, database_slug).await?;
    let authenticated_entity = unwrap_authenticated_entity(&authenticated_entity)?;

    if can_query(&authenticated_entity, &database) {
        let db_type = DBType::from_i16(database.db_type);
        let db_path = database_path(entity_slug, database_slug, &ayb_config.data_path)?;
        let result = run_query(&db_path, &query, &db_type)?;
        Ok(web::Json(result))
    } else {
        Err(AybError {
            message: format!(
                "Authenticated entity {} can not query database {}/{}",
                authenticated_entity.slug, entity_slug, database_slug
            ),
        })
    }
}

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
            if AuthenticationMethodType::from_i16(method.method_type)
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
            entity_type: EntityType::from_str(&entity_type) as i16,
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
