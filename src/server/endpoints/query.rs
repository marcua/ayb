use crate::ayb_db::db_interfaces::AybDb;
use crate::ayb_db::models::{DBType, InstantiatedEntity};

use crate::error::AybError;
use crate::hosted_db::paths::current_database_path;
use crate::hosted_db::{run_query, QueryResult};
use crate::http::structs::EntityDatabasePath;
use crate::server::config::AybConfig;
use crate::server::permissions::highest_query_access_level;
use crate::server::utils::unwrap_authenticated_entity;
use actix_web::{post, web};

#[post("/v1/{entity}/{database}/query")]
async fn query(
    path: web::Path<EntityDatabasePath>,
    query: String,
    ayb_db: web::Data<Box<dyn AybDb>>,
    ayb_config: web::Data<AybConfig>,
    authenticated_entity: Option<web::ReqData<InstantiatedEntity>>,
) -> Result<web::Json<QueryResult>, AybError> {
    let entity_slug = &path.entity.to_lowercase();
    let database_slug = &path.database;
    let database = ayb_db.get_database(entity_slug, database_slug).await?;
    let authenticated_entity = unwrap_authenticated_entity(&authenticated_entity)?;

    let access_level = highest_query_access_level(&authenticated_entity, &database)?;
    match access_level {
        Some(access_level) => {
              let db_type = DBType::try_from(database.db_type)?;
            let db_path = current_database_path(entity_slug, database_slug, &ayb_config.data_path)?;
            let result = run_query(
                &db_path,
                &query,
                &db_type,
                &ayb_config.isolation,
                access_level,
            )
            .await?;
            Ok(web::Json(result))
        }
        None => Err(AybError::Other {
            message: format!(
                "Authenticated entity {} can't query database {}/{}",
                authenticated_entity.slug, entity_slug, database_slug
            ),
        }),
    }
}
