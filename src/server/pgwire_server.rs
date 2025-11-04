use crate::ayb_db::db_interfaces::AybDb;
use crate::ayb_db::models::DBType;
use crate::error::AybError;
use crate::hosted_db::daemon_registry::DaemonRegistry;
use crate::hosted_db::paths::current_database_path;
use crate::hosted_db::{run_query, QueryMode, QueryResult};
use crate::server::config::AybConfig;
use crate::server::permissions::highest_query_access_level;

use actix_web::web;
use async_trait::async_trait;
use dyn_clone::clone_box;
use futures_util::stream;
use pgwire::api::auth::md5pass::{hash_md5_password, Md5PasswordAuthStartupHandler};
use pgwire::api::auth::{AuthSource, DefaultServerParameterProvider, LoginInfo, Password};
use pgwire::api::copy::NoopCopyHandler;
use pgwire::api::query::{PlaceholderExtendedQueryHandler, SimpleQueryHandler};
use pgwire::api::results::{DataRowEncoder, FieldFormat, FieldInfo, QueryResponse, Response};
use pgwire::api::{
    ClientInfo, NoopErrorHandler, PgWireServerHandlers, Type, METADATA_DATABASE, METADATA_USER,
};
use pgwire::error::{ErrorInfo, PgWireError, PgWireResult};
use pgwire::tokio::process_socket;

use std::sync::Arc;
use tokio::net::TcpListener;

/// Auth source that always accepts connections (for POC)
/// In production, this should validate ayb API tokens
pub struct AybAuthSource;

#[async_trait]
impl AuthSource for AybAuthSource {
    async fn get_password(&self, login_info: &LoginInfo) -> PgWireResult<Password> {
        // For POC, accept any username with dummy password
        // TODO: Validate ayb API tokens here
        let salt = vec![0, 0, 0, 0];
        let password = "dummy"; // Any password works

        let hash_password =
            hash_md5_password(login_info.user().as_ref().unwrap(), password, salt.as_ref());
        Ok(Password::new(Some(salt), hash_password.as_bytes().to_vec()))
    }
}

/// ayb's PostgreSQL wire protocol backend
pub struct AybPgWireBackend {
    ayb_db: Arc<Box<dyn AybDb>>,
    ayb_config: Arc<AybConfig>,
    daemon_registry: Arc<DaemonRegistry>,
}

impl AybPgWireBackend {
    pub fn new(
        ayb_db: Arc<Box<dyn AybDb>>,
        ayb_config: Arc<AybConfig>,
        daemon_registry: Arc<DaemonRegistry>,
    ) -> Self {
        Self {
            ayb_db,
            ayb_config,
            daemon_registry,
        }
    }

    /// Parse database path from PostgreSQL connection string
    /// Users connect with database = "marcua/test.sqlite"
    fn parse_database_path(db_name: &str) -> Result<(String, String), PgWireError> {
        let parts: Vec<&str> = db_name.split('/').collect();
        if parts.len() != 2 {
            return Err(PgWireError::UserError(Box::new(ErrorInfo::new(
                "ERROR".to_owned(),
                "XXAAA".to_owned(),
                format!(
                    "Invalid database name: {}. Use format: entity/database",
                    db_name
                ),
            ))));
        }
        Ok((parts[0].to_string(), parts[1].to_string()))
    }

    /// Execute a query against ayb database
    async fn execute_query(
        &self,
        db_name: &str,
        sql: &str,
        username: &str,
    ) -> Result<QueryResult, PgWireError> {
        // Parse entity/database from connection
        let (entity_slug, database_slug) = Self::parse_database_path(db_name)?;

        // Get database
        let database = self
            .ayb_db
            .get_database(&entity_slug, &database_slug)
            .await
            .map_err(|e| {
                PgWireError::UserError(Box::new(ErrorInfo::new(
                    "ERROR".to_owned(),
                    "42P01".to_owned(), // undefined_table
                    format!("Database not found: {}", e),
                )))
            })?;

        // Get authenticated user
        let authenticated_entity =
            (**self.ayb_db)
                .get_entity_by_slug(username)
                .await
                .map_err(|e| {
                    PgWireError::UserError(Box::new(ErrorInfo::new(
                        "ERROR".to_owned(),
                        "28000".to_owned(),
                        format!("Not authenticated: {}", e),
                    )))
                })?;

        // Check permissions
        // Wrap ayb_db in web::Data for permissions check
        let ayb_db_data = web::Data::new(clone_box(&**self.ayb_db));
        let access_level =
            highest_query_access_level(&authenticated_entity, &database, &ayb_db_data)
                .await
                .map_err(|e| {
                    PgWireError::UserError(Box::new(ErrorInfo::new(
                        "ERROR".to_owned(),
                        "42501".to_owned(), // insufficient_privilege
                        format!("Permission denied: {}", e),
                    )))
                })?;

        let access_level = access_level.ok_or_else(|| {
            PgWireError::UserError(Box::new(ErrorInfo::new(
                "ERROR".to_owned(),
                "42501".to_owned(),
                "You don't have access to this database".to_owned(),
            )))
        })?;

        // Determine query mode from SQL
        // Note: query_mode is determined from access_level and SQL type
        let _query_mode = if sql.trim().to_uppercase().starts_with("SELECT")
            || sql.trim().to_uppercase().starts_with("WITH")
            || sql.trim().to_uppercase().starts_with("EXPLAIN")
        {
            QueryMode::ReadOnly
        } else {
            QueryMode::ReadWrite
        };

        // Execute query
        let db_type = DBType::try_from(database.db_type).map_err(|e| {
            PgWireError::UserError(Box::new(ErrorInfo::new(
                "ERROR".to_owned(),
                "XX000".to_owned(),
                format!("Invalid database type: {}", e),
            )))
        })?;

        let db_path =
            current_database_path(&entity_slug, &database_slug, &self.ayb_config.data_path)
                .map_err(|e| {
                    PgWireError::UserError(Box::new(ErrorInfo::new(
                        "ERROR".to_owned(),
                        "XX000".to_owned(),
                        format!("Database path error: {}", e),
                    )))
                })?;

        let result = run_query(
            &self.daemon_registry,
            &db_path,
            sql,
            &db_type,
            &self.ayb_config.isolation,
            access_level,
        )
        .await
        .map_err(|e| {
            PgWireError::UserError(Box::new(ErrorInfo::new(
                "ERROR".to_owned(),
                "42601".to_owned(), // syntax_error (or appropriate code)
                format!("Query error: {}", e),
            )))
        })?;

        Ok(result)
    }

    /// Convert ayb QueryResult to PostgreSQL wire format
    fn encode_query_result(result: QueryResult) -> PgWireResult<Vec<Response<'static>>> {
        // Build field info from column names
        let fields: Vec<FieldInfo> = result
            .fields
            .iter()
            .map(|name| {
                // For simplicity, treat all fields as TEXT
                // In real implementation, infer types from data
                FieldInfo::new(name.clone(), None, None, Type::TEXT, FieldFormat::Text)
            })
            .collect();

        let schema = Arc::new(fields);
        let mut rows_data = Vec::new();

        // Encode each row
        for row in &result.rows {
            let mut encoder = DataRowEncoder::new(schema.clone());
            for cell in row {
                match cell {
                    Some(value) => encoder.encode_field(value)?,
                    None => encoder.encode_field(&None::<String>)?,
                }
            }
            rows_data.push(encoder.finish());
        }

        Ok(vec![Response::Query(QueryResponse::new(
            schema,
            stream::iter(rows_data),
        ))])
    }
}

/// Implement SimpleQueryHandler for basic SQL queries
#[async_trait]
impl SimpleQueryHandler for AybPgWireBackend {
    async fn do_query<'a, C>(
        &self,
        client: &mut C,
        query: &'a str,
    ) -> PgWireResult<Vec<Response<'a>>>
    where
        C: ClientInfo + Unpin + Send + Sync,
    {
        // Get database name and username from client connection
        let db_name = client.metadata().get(METADATA_DATABASE).ok_or_else(|| {
            PgWireError::UserError(Box::new(ErrorInfo::new(
                "ERROR".to_owned(),
                "08P01".to_owned(),
                "No database specified in connection".to_owned(),
            )))
        })?;

        let username = client.metadata().get(METADATA_USER).ok_or_else(|| {
            PgWireError::UserError(Box::new(ErrorInfo::new(
                "ERROR".to_owned(),
                "28000".to_owned(),
                "No username in connection".to_owned(),
            )))
        })?;

        // Execute query
        let result = self.execute_query(db_name, query, username).await?;

        // Convert to PostgreSQL format
        Self::encode_query_result(result)
    }
}

/// Factory for creating backend instances
pub struct AybPgWireBackendFactory {
    ayb_db: Arc<Box<dyn AybDb>>,
    ayb_config: Arc<AybConfig>,
    daemon_registry: Arc<DaemonRegistry>,
}

impl AybPgWireBackendFactory {
    pub fn new(
        ayb_db: Arc<Box<dyn AybDb>>,
        ayb_config: Arc<AybConfig>,
        daemon_registry: Arc<DaemonRegistry>,
    ) -> Self {
        Self {
            ayb_db,
            ayb_config,
            daemon_registry,
        }
    }
}

impl PgWireServerHandlers for AybPgWireBackendFactory {
    type StartupHandler =
        Md5PasswordAuthStartupHandler<AybAuthSource, DefaultServerParameterProvider>;
    type SimpleQueryHandler = AybPgWireBackend;
    type ExtendedQueryHandler = PlaceholderExtendedQueryHandler;
    type CopyHandler = NoopCopyHandler;
    type ErrorHandler = NoopErrorHandler;

    fn simple_query_handler(&self) -> Arc<Self::SimpleQueryHandler> {
        Arc::new(AybPgWireBackend::new(
            Arc::clone(&self.ayb_db),
            Arc::clone(&self.ayb_config),
            Arc::clone(&self.daemon_registry),
        ))
    }

    fn extended_query_handler(&self) -> Arc<Self::ExtendedQueryHandler> {
        Arc::new(PlaceholderExtendedQueryHandler)
    }

    fn startup_handler(&self) -> Arc<Self::StartupHandler> {
        let parameters = DefaultServerParameterProvider::default();
        Arc::new(Md5PasswordAuthStartupHandler::new(
            Arc::new(AybAuthSource),
            Arc::new(parameters),
        ))
    }

    fn copy_handler(&self) -> Arc<Self::CopyHandler> {
        Arc::new(NoopCopyHandler)
    }

    fn error_handler(&self) -> Arc<Self::ErrorHandler> {
        Arc::new(NoopErrorHandler)
    }
}

/// Start PostgreSQL wire protocol server
pub async fn start_pgwire_server(
    ayb_db: Arc<Box<dyn AybDb>>,
    ayb_config: Arc<AybConfig>,
    daemon_registry: Arc<DaemonRegistry>,
    host: &str,
    port: u16,
) -> Result<(), AybError> {
    let addr = format!("{}:{}", host, port);
    let listener = TcpListener::bind(&addr).await?;

    println!("PostgreSQL wire protocol server listening on {}", addr);
    println!(
        "Connect with: psql -h {} -p {} -d entity/database -U username",
        host, port
    );

    let factory = Arc::new(AybPgWireBackendFactory::new(
        ayb_db,
        ayb_config,
        daemon_registry,
    ));

    loop {
        let (socket, _addr) = listener.accept().await?;
        let factory = Arc::clone(&factory);

        tokio::spawn(async move {
            if let Err(e) = process_socket(socket, None, factory).await {
                eprintln!("Error processing pgwire connection: {}", e);
            }
        });
    }
}
