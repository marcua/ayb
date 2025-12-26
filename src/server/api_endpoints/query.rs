use crate::ayb_db::db_interfaces::AybDb;
use crate::ayb_db::models::{APIToken, DBType, InstantiatedEntity};

use crate::error::AybError;
use crate::hosted_db::daemon_registry::DaemonRegistry;
use crate::hosted_db::paths::current_database_path;
use crate::hosted_db::{run_query, QueryResult};
use crate::http::structs::EntityDatabasePath;
use crate::server::config::AybConfig;
use crate::server::permissions::highest_query_access_level_with_token;
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
    api_token: Option<web::ReqData<APIToken>>,
) -> Result<web::Json<QueryResult>, AybError> {
    let entity_slug = &path.entity.to_lowercase();
    let database_slug = &path.database;
    let database = ayb_db.get_database(entity_slug, database_slug).await?;
    let authenticated_entity = unwrap_authenticated_entity(&authenticated_entity)?;
    let token_ref = api_token.as_ref().map(|t| t.clone().into_inner());
    let token = token_ref.as_ref();

    let access_level =
        highest_query_access_level_with_token(&authenticated_entity, &database, token, &ayb_db)
            .await?;
    match access_level {
        Some(access_level) => {
            let db_type = DBType::try_from(database.db_type)?;
            let db_path = current_database_path(entity_slug, database_slug, &ayb_config.data_path)?;
            let result = run_query(
                &daemon_registry,
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
