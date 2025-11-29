use crate::ayb_db::db_interfaces::AybDb;
use crate::ayb_db::models::{DBType, InstantiatedEntity};
use crate::error::AybError;
use crate::hosted_db::daemon_registry::DaemonRegistry;
use crate::hosted_db::paths::current_database_path;
use crate::hosted_db::{run_query, QueryResult};
use crate::server::config::AybConfig;
use crate::server::permissions::highest_query_access_level;
use actix_web::web;

/// Execute a query with authentication and permission checks.
///
/// This is the core query execution logic shared between HTTP and pgwire endpoints.
/// It handles:
/// - Fetching the database
/// - Checking permissions
/// - Executing the query with appropriate access level
///
/// # Arguments
/// * `authenticated_entity` - The entity making the request (already authenticated)
/// * `entity_slug` - The entity that owns the database
/// * `database_slug` - The database name
/// * `query` - The SQL query to execute
/// * `ayb_db` - Database interface
/// * `ayb_config` - Server configuration
/// * `daemon_registry` - Daemon registry for query execution
///
/// # Returns
/// The query result or an error
pub async fn execute_authenticated_query(
    authenticated_entity: &InstantiatedEntity,
    entity_slug: &str,
    database_slug: &str,
    query: &str,
    ayb_db: &web::Data<Box<dyn AybDb>>,
    ayb_config: &AybConfig,
    daemon_registry: &DaemonRegistry,
) -> Result<QueryResult, AybError> {
    // Get the database
    let database = ayb_db.get_database(entity_slug, database_slug).await?;

    // Check permissions - this handles read-only vs read-write access
    let access_level = highest_query_access_level(authenticated_entity, &database, ayb_db).await?;

    let access_level = access_level.ok_or_else(|| AybError::Other {
        message: format!(
            "Authenticated entity {} can't query database {}/{}",
            authenticated_entity.slug, entity_slug, database_slug
        ),
    })?;

    // Get database type and path
    let db_type = DBType::try_from(database.db_type)?;
    let db_path = current_database_path(entity_slug, database_slug, &ayb_config.data_path)?;

    // Execute the query with the appropriate access level
    run_query(
        daemon_registry,
        &db_path,
        query,
        &db_type,
        &ayb_config.isolation,
        access_level,
    )
    .await
}
