use crate::error::StacksError;
use crate::hosted_db::{run_query, QueryResult};
use crate::http::structs::{EntityDatabasePath, EntityPath};
use crate::http::utils::get_header;
use crate::stacks_db::crud::{
    create_database as create_database_crud, create_entity as create_entity_crud,
    get_database as get_database_crud, get_entity as get_entity_crud,
};
use crate::stacks_db::models::{
    DBType, Database, Entity, EntityType, InstantiatedDatabase, InstantiatedEntity,
};
use actix_web::{post, web, HttpRequest};
use sqlx;

#[post("/v1/{entity}/{database}")]
async fn create_database(
    path: web::Path<EntityDatabasePath>,
    req: HttpRequest,
    db_pool: web::Data<sqlx::PgPool>,
) -> Result<web::Json<InstantiatedDatabase>, StacksError> {
    let entity_slug = &path.entity;
    let entity = get_entity_crud(entity_slug, &db_pool).await?;
    let db_type = get_header(req, "db-type")?;
    let database = Database {
        entity_id: entity.id,
        slug: path.database.clone(),
        db_type: DBType::from_str(&db_type) as i16,
    };
    let created_database = create_database_crud(&database, &db_pool).await?;
    Ok(web::Json(created_database))
}

#[post("/v1/{entity}")]
async fn create_entity(
    path: web::Path<EntityPath>,
    req: HttpRequest,
    db_pool: web::Data<sqlx::PgPool>,
) -> Result<web::Json<InstantiatedEntity>, StacksError> {
    let entity_type = get_header(req, "entity-type")?;
    let entity = Entity {
        slug: path.entity.clone(),
        entity_type: EntityType::from_str(&entity_type) as i16,
    };
    let created_entity = create_entity_crud(&entity, &db_pool).await?;
    Ok(web::Json(created_entity))
}

#[post("/v1/{entity}/{database}/query")]
async fn query(
    path: web::Path<EntityDatabasePath>,
    query: String,
    db_pool: web::Data<sqlx::PgPool>,
) -> Result<web::Json<QueryResult>, StacksError> {
    let entity_slug = &path.entity;
    let database_slug = &path.database;
    let database = get_database_crud(entity_slug, database_slug, &db_pool).await?;
    let db_type = DBType::from_i16(database.db_type);
    // TODO(marcua): make the path relate to some
    // persistent storage (with high availability, etc.)
    let path = ["/tmp", entity_slug, database_slug].iter().collect();
    let result = run_query(&path, &query, &db_type)?;
    Ok(web::Json(result))
}
