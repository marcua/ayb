use crate::ayb_db::db_interfaces::AybDb;
use crate::ayb_db::models::{DBType, Database, InstantiatedEntity};
use std::str::FromStr;

use crate::error::AybError;

use crate::hosted_db::paths::database_path;
use crate::http::permissions::can_create_database;
use crate::http::structs::{AybConfig, Database as APIDatabase, EntityDatabasePath};
use crate::http::utils::{get_header, unwrap_authenticated_entity};
use actix_web::{post, web, HttpRequest, HttpResponse};

#[post("/v1/{entity}/{database}/create")]
async fn create_database(
    path: web::Path<EntityDatabasePath>,
    req: HttpRequest,
    ayb_db: web::Data<Box<dyn AybDb>>,
    ayb_config: web::Data<AybConfig>,
    authenticated_entity: Option<web::ReqData<InstantiatedEntity>>,
) -> Result<HttpResponse, AybError> {
    let entity_slug = &path.entity;
    let entity = ayb_db.get_entity_by_slug(entity_slug).await?;
    let db_type = get_header(&req, "db-type")?;
    let database = Database {
        entity_id: entity.id,
        slug: path.database.clone(),
        db_type: DBType::from_str(&db_type)? as i16,
    };
    let authenticated_entity = unwrap_authenticated_entity(&authenticated_entity)?;
    if can_create_database(&authenticated_entity, &entity) {
        let created_database = ayb_db.create_database(&database).await?;
        // Create the database file at the appropriate path
        let _ = database_path(entity_slug, &path.database, &ayb_config.data_path, true)?;
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
