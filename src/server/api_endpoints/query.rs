use crate::ayb_db::db_interfaces::AybDb;
use crate::ayb_db::models::InstantiatedEntity;
use crate::error::AybError;
use crate::hosted_db::daemon_registry::DaemonRegistry;
use crate::hosted_db::QueryResult;
use crate::http::structs::EntityDatabasePath;
use crate::server::config::AybConfig;
use crate::server::query_execution::execute_authenticated_query;
use crate::server::utils::unwrap_authenticated_entity;
use actix_web::{post, web};

#[post("/{entity}/{database}/query")]
async fn query(
    path: web::Path<EntityDatabasePath>,
    query: String,
    ayb_db: web::Data<Box<dyn AybDb>>,
    ayb_config: web::Data<AybConfig>,
    daemon_registry: web::Data<DaemonRegistry>,
    authenticated_entity: Option<web::ReqData<InstantiatedEntity>>,
) -> Result<web::Json<QueryResult>, AybError> {
    let entity_slug = &path.entity.to_lowercase();
    let database_slug = &path.database;
    let authenticated_entity = unwrap_authenticated_entity(&authenticated_entity)?;

    // Execute query using shared logic
    let result = execute_authenticated_query(
        &authenticated_entity,
        entity_slug,
        database_slug,
        &query,
        &ayb_db,
        &ayb_config,
        &daemon_registry,
    )
    .await?;

    Ok(web::Json(result))
}
