use crate::ayb_db::db_interfaces::AybDb;
use crate::ayb_db::models::{APIToken, DBType, InstantiatedEntity};

use crate::error::AybError;
use crate::hosted_db::daemon_registry::DaemonRegistry;
use crate::hosted_db::paths::current_database_path;
use crate::hosted_db::{run_query, QueryResult};
use crate::http::structs::{EntityDatabasePath, QueryRequest};
use crate::server::config::AybConfig;
use crate::server::permissions::highest_query_access_level;
use crate::server::utils::unwrap_authenticated_entity;
use actix_web::http::header;
use actix_web::{post, web, HttpRequest};

#[post(
    "/{entity}/{database}/query",
    wrap = "actix_web_httpauth::middleware::HttpAuthentication::bearer(crate::server::server_runner::entity_validator)"
)]
#[allow(clippy::too_many_arguments)]
async fn query(
    req: HttpRequest,
    path: web::Path<EntityDatabasePath>,
    body: String,
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

    // Negotiate on Content-Type: a JSON body carries `{query, params}`,
    // while any other body (the historical behavior) is treated as bare
    // SQL with no parameters. This keeps existing plain-text clients
    // working unchanged.
    let is_json = req
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .map(|value| value.starts_with("application/json"))
        .unwrap_or(false);
    let (query, params) = if is_json {
        let parsed: QueryRequest =
            serde_json::from_str(&body).map_err(|err| AybError::QueryError {
                message: format!("Unable to parse JSON query request: {err}"),
            })?;
        (parsed.query, parsed.params)
    } else {
        (body, Vec::new())
    };

    let access_level =
        highest_query_access_level(&authenticated_entity, &database, token, &ayb_db).await?;
    match access_level {
        Some(access_level) => {
            let db_type = DBType::try_from(database.db_type)?;
            let db_path = current_database_path(entity_slug, database_slug, &ayb_config.data_path)?;
            let result = run_query(
                &daemon_registry,
                &db_path,
                &query,
                &params,
                &db_type,
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
