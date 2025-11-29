use crate::ayb_db::db_interfaces::AybDb;
use crate::error::AybError;
use crate::hosted_db::daemon_registry::DaemonRegistry;
use crate::hosted_db::QueryResult;
use crate::server::config::AybConfig;
use crate::server::query_execution::execute_authenticated_query;
use crate::server::tokens::retrieve_and_validate_api_token;

use actix_web::web;
use async_trait::async_trait;
use dyn_clone::clone_box;
use futures_util::sink::{Sink, SinkExt};
use futures_util::stream;
use pgwire::api::auth::{
    finish_authentication, save_startup_parameters_to_metadata, DefaultServerParameterProvider,
    LoginInfo, StartupHandler,
};
use pgwire::api::query::SimpleQueryHandler;
use pgwire::api::results::{DataRowEncoder, FieldFormat, FieldInfo, QueryResponse, Response};
use pgwire::api::{
    ClientInfo, PgWireConnectionState, PgWireServerHandlers, Type, METADATA_DATABASE, METADATA_USER,
};
use pgwire::error::{ErrorInfo, PgWireError, PgWireResult};
use pgwire::messages::response::ErrorResponse;
use pgwire::messages::startup::Authentication;
use pgwire::messages::{PgWireBackendMessage, PgWireFrontendMessage};
use pgwire::tokio::process_socket;

use std::fmt::Debug;
use std::sync::Arc;
use tokio::net::TcpListener;

/// PostgreSQL error severity levels
const SEVERITY_ERROR: &str = "ERROR";
const SEVERITY_FATAL: &str = "FATAL";

/// PostgreSQL error codes used in this module
mod error_codes {
    /// Authentication failure
    pub const INVALID_AUTH: &str = "28P01";
    /// Invalid authorization specification
    pub const INVALID_AUTH_SPEC: &str = "28000";
    /// Protocol violation
    pub const PROTOCOL_VIOLATION: &str = "08P01";
    /// Syntax error (generic SQL error)
    pub const SYNTAX_ERROR: &str = "42601";
    /// Custom error code for invalid database path format
    pub const INVALID_DB_PATH: &str = "XXAAA";
}

/// Create a PgWireError with the given severity, code, and message
fn pgwire_error(severity: &str, code: &str, message: impl Into<String>) -> PgWireError {
    PgWireError::UserError(Box::new(ErrorInfo::new(
        severity.to_owned(),
        code.to_owned(),
        message.into(),
    )))
}

/// Send a fatal error to the client and close the connection
async fn send_fatal_error_and_close<C>(
    client: &mut C,
    code: &str,
    message: impl Into<String>,
) -> PgWireResult<()>
where
    C: Sink<PgWireBackendMessage> + Unpin + Send,
    PgWireError: From<<C as Sink<PgWireBackendMessage>>::Error>,
{
    let error_info = ErrorInfo::new(SEVERITY_FATAL.to_owned(), code.to_owned(), message.into());
    let error = ErrorResponse::from(error_info);
    client
        .feed(PgWireBackendMessage::ErrorResponse(error))
        .await?;
    client.close().await?;
    Ok(())
}

/// Custom startup handler that validates ayb API tokens as passwords
pub struct AybTokenAuthStartupHandler {
    ayb_db: Arc<Box<dyn AybDb>>,
    parameter_provider: Arc<DefaultServerParameterProvider>,
}

impl AybTokenAuthStartupHandler {
    pub fn new(
        ayb_db: Arc<Box<dyn AybDb>>,
        parameter_provider: Arc<DefaultServerParameterProvider>,
    ) -> Self {
        Self {
            ayb_db,
            parameter_provider,
        }
    }

    /// Validate an API token and verify it belongs to the given username.
    /// Returns Ok(()) on success, or Err(message) with an error message on failure.
    async fn validate_token_and_authenticate(
        &self,
        token: &str,
        username: &str,
    ) -> Result<(), String> {
        let ayb_db_data = web::Data::new(clone_box(&**self.ayb_db));

        let api_token = retrieve_and_validate_api_token(token, &ayb_db_data)
            .await
            .map_err(|_| "Invalid API token".to_string())?;

        let entity = (**self.ayb_db)
            .get_entity_by_id(api_token.entity_id)
            .await
            .map_err(|_| "Invalid API token: entity not found".to_string())?;

        if entity.slug.to_lowercase() != username.to_lowercase() {
            return Err(format!(
                "Token belongs to entity '{}', but connected as '{}'",
                entity.slug, username
            ));
        }

        Ok(())
    }
}

#[async_trait]
impl StartupHandler for AybTokenAuthStartupHandler {
    async fn on_startup<C>(
        &self,
        client: &mut C,
        message: PgWireFrontendMessage,
    ) -> PgWireResult<()>
    where
        C: ClientInfo + Sink<PgWireBackendMessage> + Unpin + Send,
        C::Error: Debug,
        PgWireError: From<<C as Sink<PgWireBackendMessage>>::Error>,
    {
        match message {
            PgWireFrontendMessage::Startup(ref startup) => {
                save_startup_parameters_to_metadata(client, startup);
                client.set_state(PgWireConnectionState::AuthenticationInProgress);

                // Request cleartext password (which will be the ayb API token)
                client
                    .send(PgWireBackendMessage::Authentication(
                        Authentication::CleartextPassword,
                    ))
                    .await?;
            }
            PgWireFrontendMessage::PasswordMessageFamily(pwd) => {
                let pwd = pwd.into_password()?;
                let login_info = LoginInfo::from_client_info(client);
                let username = login_info.user().ok_or_else(|| {
                    pgwire_error(
                        SEVERITY_FATAL,
                        error_codes::INVALID_AUTH_SPEC,
                        "No username provided",
                    )
                })?;

                // The password should be an ayb API token
                let token = pwd.password;

                // Validate token and authenticate
                let auth_result = self.validate_token_and_authenticate(&token, username).await;

                match auth_result {
                    Ok(()) => {
                        finish_authentication(client, self.parameter_provider.as_ref()).await?;
                    }
                    Err(message) => {
                        send_fatal_error_and_close(client, error_codes::INVALID_AUTH, message)
                            .await?;
                    }
                }
            }
            _ => {}
        }
        Ok(())
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
            return Err(pgwire_error(
                SEVERITY_ERROR,
                error_codes::INVALID_DB_PATH,
                format!(
                    "Invalid database name: {}. Use format: entity/database",
                    db_name
                ),
            ));
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

        // Get authenticated user
        let authenticated_entity =
            (**self.ayb_db)
                .get_entity_by_slug(username)
                .await
                .map_err(|e| {
                    pgwire_error(
                        SEVERITY_ERROR,
                        error_codes::INVALID_AUTH_SPEC,
                        format!("Not authenticated: {}", e),
                    )
                })?;

        // Wrap ayb_db in web::Data for shared query execution logic
        let ayb_db_data = web::Data::new(clone_box(&**self.ayb_db));

        // Execute query using shared logic
        execute_authenticated_query(
            &authenticated_entity,
            &entity_slug,
            &database_slug,
            sql,
            &ayb_db_data,
            &self.ayb_config,
            &self.daemon_registry,
        )
        .await
        .map_err(|e| pgwire_error(SEVERITY_ERROR, error_codes::SYNTAX_ERROR, e.to_string()))
    }

    /// Convert ayb QueryResult to PostgreSQL wire format
    fn encode_query_result(result: QueryResult) -> PgWireResult<Vec<Response>> {
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
    async fn do_query<C>(&self, client: &mut C, query: &str) -> PgWireResult<Vec<Response>>
    where
        C: ClientInfo + Unpin + Send + Sync,
    {
        // Get database name and username from client connection
        let db_name = client.metadata().get(METADATA_DATABASE).ok_or_else(|| {
            pgwire_error(
                SEVERITY_ERROR,
                error_codes::PROTOCOL_VIOLATION,
                "No database specified in connection",
            )
        })?;

        let username = client.metadata().get(METADATA_USER).ok_or_else(|| {
            pgwire_error(
                SEVERITY_ERROR,
                error_codes::INVALID_AUTH_SPEC,
                "No username in connection",
            )
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
    fn simple_query_handler(&self) -> Arc<impl SimpleQueryHandler> {
        Arc::new(AybPgWireBackend::new(
            Arc::clone(&self.ayb_db),
            Arc::clone(&self.ayb_config),
            Arc::clone(&self.daemon_registry),
        ))
    }

    fn startup_handler(&self) -> Arc<impl StartupHandler> {
        let parameters = DefaultServerParameterProvider::default();
        Arc::new(AybTokenAuthStartupHandler::new(
            Arc::clone(&self.ayb_db),
            Arc::new(parameters),
        ))
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
    println!("Use your ayb API token as the password");

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
