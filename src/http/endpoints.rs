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
    AybConfig, Database as APIDatabase, Entity as APIEntity, EntityDatabasePath, EntityPath,
};
use crate::http::utils::get_header;
use actix_web::{post, web, HttpRequest, HttpResponse};

#[post("/v1/{entity}/{database}")]
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

#[post("/v1/{entity}")]
async fn register(
    path: web::Path<EntityPath>,
    req: HttpRequest,
    ayb_db: web::Data<Box<dyn AybDb>>,
    ayb_config: web::Data<AybConfig>,
) -> Result<HttpResponse, AybError> {
    let email_address = get_header(&req, "email-address")?;
    let entity_type = get_header(&req, "entity-type")?;
    println!("Getting or creating entity");
    let created_entity = ayb_db
        .get_or_create_entity(&Entity {
            slug: path.entity.clone(),
            entity_type: EntityType::from_str(&entity_type) as i16,
        })
        .await?;
    // Ensure that there are no verified authentication methods, and
    // check to see if this method has been previously attempted but
    // not verified.
    println!("Listing");
    let authentication_methods = ayb_db.list_authentication_methods(&created_entity).await?;
    let mut already_verified = false;
    let mut authentication_method: Option<InstantiatedAuthenticationMethod> = None;
    for method in authentication_methods {
        if method.status == (AuthenticationMethodStatus::Verified as i16) {
            already_verified = true;
            break;
        }
        if method.status == (AuthenticationMethodStatus::Unverified as i16)
            && method.method_type == (AuthenticationMethodType::Email as i16)
            && method.email_address == email_address
        {
            authentication_method = Some(method)
        }
    }

    println!("Pre-verification");
    if already_verified {
        return Err(AybError {
            message: format!("This entity has already been registered"),
        });
    }

    if let None = authentication_method {
        println!("Creating auth method");
        authentication_method = Some(
            ayb_db
                .create_authentication_method(&AuthenticationMethod {
                    entity_id: created_entity.id,
                    method_type: AuthenticationMethodType::Email as i16,
                    status: AuthenticationMethodStatus::Unverified as i16,
                    email_address: email_address.to_owned(),
                })
                .await?,
        );
    }

    // send_registration_email(&email_address, "fake token", &ayb_config.email).await?;
    Ok(HttpResponse::Created().json(APIEntity::from_persisted(&created_entity)))
}
