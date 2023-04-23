use crate::ayb_db::crud::{
    create_database as create_database_crud, create_entity as create_entity_crud,
    get_database as get_database_crud, get_entity as get_entity_crud,
};
use crate::ayb_db::models::{DBType, Database, Entity, EntityType};
use crate::error::AybError;
use crate::hosted_db::paths::database_path;
use crate::hosted_db::{run_query, QueryResult};
use crate::http::structs::{
    AybConfig, Database as APIDatabase, Entity as APIEntity, EntityDatabasePath, EntityPath,
};
use crate::http::utils::get_header;
use actix_web::{post, web, HttpRequest, HttpResponse};
use sqlx;

#[post("/v1/{entity}/{database}")]
async fn create_database(
    path: web::Path<EntityDatabasePath>,
    req: HttpRequest,
    db_pool: web::Data<sqlx::PgPool>,
) -> Result<HttpResponse, AybError> {
    let entity_slug = &path.entity;
    let entity = get_entity_crud(entity_slug, &db_pool).await?;
    let db_type = get_header(req, "db-type")?;
    let database = Database {
        entity_id: entity.id,
        slug: path.database.clone(),
        db_type: DBType::from_str(&db_type) as i16,
    };
    let created_database = create_database_crud(&database, &db_pool).await?;
    Ok(HttpResponse::Created().json(APIDatabase::from_persisted(&entity, &created_database)))
}

#[post("/v1/{entity}/{database}/query")]
async fn query(
    path: web::Path<EntityDatabasePath>,
    query: String,
    db_pool: web::Data<sqlx::PgPool>,
    ayb_config: web::Data<AybConfig>,
) -> Result<web::Json<QueryResult>, AybError> {
    let entity_slug = &path.entity;
    let database_slug = &path.database;
    let database = get_database_crud(entity_slug, database_slug, &db_pool).await?;
    let db_type = DBType::from_i16(database.db_type);
    let db_path = database_path(entity_slug, database_slug, &ayb_config.data_path)?;
    let result = run_query(&db_path, &query, &db_type)?;
    Ok(web::Json(result))
}

#[post("/v1/{entity}")]
async fn register(
    path: web::Path<EntityPath>,
    req: HttpRequest,
    db_pool: web::Data<sqlx::PgPool>,
) -> Result<HttpResponse, AybError> {
    let entity_type = get_header(req, "entity-type")?;
    let entity = Entity {
        slug: path.entity.clone(),
        entity_type: EntityType::from_str(&entity_type) as i16,
    };
    let created_entity = create_entity_crud(&entity, &db_pool).await?;
    Ok(HttpResponse::Created().json(APIEntity::from_persisted(&created_entity)))
}
