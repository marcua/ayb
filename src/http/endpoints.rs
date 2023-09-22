use crate::ayb_db::db_interfaces::AybDb;
use crate::ayb_db::models::{
    AuthenticationMethod, AuthenticationMethodStatus, AuthenticationMethodType, DBType, Database,
    Entity, EntityType, InstantiatedAuthenticationMethod,
};
use crate::email::send_registration_email;
use crate::error::AybError;
use crate::hosted_db::paths::database_path;
use crate::hosted_db::{run_query, QueryResult};
use crate::http::structs::{
    APIKey as APIAPIKey, AuthenticationDetails, AybConfig, Database as APIDatabase, EmptyResponse,
    EntityDatabasePath,
};
use crate::http::tokens::{decrypt_auth_token, encrypt_auth_token};
use crate::http::utils::{get_header, get_lowercased_header};
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

    // TODO(marcua): When we implement permissions, get_or_create default API keys.
    // Ok(HttpResponse::Ok().json(APIAPIKey::from_persisted(&created_key)))
    Ok(HttpResponse::Ok().json(APIAPIKey {
        name: "default".to_string(),
        key: "insecure, unimplemented".to_string(),
    }))
}

#[post("/v1/{entity}/{database}/create")]
async fn create_database(
    path: web::Path<EntityDatabasePath>,
    req: HttpRequest,
    ayb_db: web::Data<Box<dyn AybDb>>,
) -> Result<HttpResponse, AybError> {
    let entity_slug = &path.entity;
    let entity = ayb_db.get_entity(entity_slug).await?;
    let db_type = get_header(&req, "db-type")?;
    let database = Database {
        entity_id: entity.id,
        slug: path.database.clone(),
        db_type: DBType::from_str(&db_type) as i16,
    };
    let created_database = ayb_db.create_database(&database).await?;
    Ok(HttpResponse::Created().json(APIDatabase::from_persisted(&entity, &created_database)))
}

#[post("/v1/log_in")]
async fn log_in(
    req: HttpRequest,
    ayb_db: web::Data<Box<dyn AybDb>>,
    ayb_config: web::Data<AybConfig>,
) -> Result<HttpResponse, AybError> {
    let entity = get_lowercased_header(&req, "entity")?;
    let desired_entity = ayb_db.get_entity(&entity).await;

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
) -> Result<web::Json<QueryResult>, AybError> {
    let entity_slug = &path.entity;
    let database_slug = &path.database;
    let database = ayb_db.get_database(entity_slug, database_slug).await?;
    let db_type = DBType::from_i16(database.db_type);
    let db_path = database_path(entity_slug, database_slug, &ayb_config.data_path)?;
    let result = run_query(&db_path, &query, &db_type)?;
    Ok(web::Json(result))
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
    let desired_entity = ayb_db.get_entity(&entity).await;
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
