use crate::ayb_db::models::{
    APITokenWithDatabase, DBType, DatabasePermission, EntityDatabaseSharingLevel, EntityType,
    InstantiatedDatabase as PersistedDatabase, InstantiatedDatabase,
    InstantiatedEntity as PersistedEntity,
};
use crate::formatting::TabularFormatter;
use crate::hosted_db::QueryMode;
use crate::server::snapshots::models::ListSnapshotResult;
use prettytable::{Cell, Row, Table};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct Database {
    pub entity: String,
    pub database: String,
    pub database_type: String,
}

impl Database {
    pub fn from_persisted(entity: &PersistedEntity, database: &PersistedDatabase) -> Database {
        Database {
            entity: entity.slug.clone(),
            database: database.slug.clone(),
            database_type: DBType::try_from(database.db_type)
                .expect("unknown database type")
                .to_str()
                .to_string(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Entity {
    pub entity: String,
    pub entity_type: String,
}

impl Entity {
    pub fn from_persisted(entity: &PersistedEntity) -> Entity {
        Entity {
            entity: entity.slug.clone(),
            entity_type: EntityType::try_from(entity.entity_type)
                .expect("unknown entity type")
                .to_str()
                .to_string(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct EntityDatabasePath {
    pub entity: String,
    pub database: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct EntityPath {
    pub entity: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProfileLinkUpdate {
    pub url: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct EntityProfileLink {
    pub url: String,
    pub verified: bool,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct EntityProfile {
    pub display_name: Option<String>,
    pub description: Option<String>,
    pub organization: Option<String>,
    pub location: Option<String>,
    pub links: Vec<EntityProfileLink>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct EntityPermissions {
    pub can_create_database: bool,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct EntityQueryResponse {
    pub slug: String,
    pub profile: EntityProfile,
    pub databases: Vec<EntityDatabase>,
    pub permissions: EntityPermissions,
}

impl TabularFormatter for EntityProfile {
    fn to_table(&self) -> Table {
        let mut table = Table::new();
        table.set_titles(Row::new(vec![
            Cell::new("Display name"),
            Cell::new("Description"),
            Cell::new("Organization"),
            Cell::new("Location"),
            Cell::new("Links"),
        ]));

        table.add_row(Row::new(vec![
            Cell::new(self.display_name.as_deref().unwrap_or("null")),
            Cell::new(self.description.as_deref().unwrap_or("null")),
            Cell::new(self.organization.as_deref().unwrap_or("null")),
            Cell::new(self.location.as_deref().unwrap_or("null")),
            Cell::new(
                &self
                    .links
                    .clone()
                    .into_iter()
                    .map(|v| {
                        if v.verified {
                            format!("{} (verified)", v.url)
                        } else {
                            v.url
                        }
                    })
                    .collect::<Vec<String>>()
                    .join(","),
            ),
        ]));

        table
    }
}

impl TabularFormatter for Vec<EntityDatabase> {
    fn to_table(&self) -> Table {
        let mut table = Table::new();
        table.set_titles(Row::new(vec![
            Cell::new("Database slug"),
            Cell::new("Type"),
        ]));

        self.iter()
            .map(|v| Row::new(vec![Cell::new(&v.slug), Cell::new(&v.database_type)]))
            .for_each(|c| {
                table.add_row(c);
            });

        table
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct EntityDatabase {
    pub slug: String,
    pub database_type: String,
}

impl From<InstantiatedDatabase> for EntityDatabase {
    fn from(value: InstantiatedDatabase) -> Self {
        Self {
            slug: value.slug,
            database_type: DBType::try_from(value.db_type).unwrap().to_str().into(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AuthenticationDetails {
    pub version: u16,
    pub entity: String,
    pub entity_type: i16,
    pub email_address: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct APIToken {
    pub entity: String,
    pub token: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EmptyResponse {}

#[derive(Debug, Serialize, Deserialize)]
pub struct HealthResponse {
    pub status: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SnapshotList {
    pub snapshots: Vec<ListSnapshotResult>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DatabasePermissions {
    pub permissions: Vec<DatabasePermission>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DatabaseDetails {
    pub entity_slug: String,
    pub database_slug: String,
    pub database_type: String,
    pub highest_query_access_level: Option<QueryMode>,
    pub can_manage_database: bool,
    pub public_sharing_level: String,
}

impl TabularFormatter for Vec<ListSnapshotResult> {
    fn to_table(&self) -> Table {
        let mut table = Table::new();
        table.set_titles(Row::new(vec![
            Cell::new("Name"),
            Cell::new("Last modified"),
        ]));

        self.iter()
            .map(|v| {
                Row::new(vec![
                    Cell::new(&v.snapshot_id),
                    Cell::new(&v.last_modified_at.to_rfc3339()),
                ])
            })
            .for_each(|c| {
                table.add_row(c);
            });

        table
    }
}

impl TabularFormatter for Vec<DatabasePermission> {
    fn to_table(&self) -> Table {
        let mut table = Table::new();
        table.set_titles(Row::new(vec![
            Cell::new("Entity"),
            Cell::new("Sharing level"),
        ]));

        self.iter()
            .map(|v| Row::new(vec![Cell::new(&v.entity_slug), Cell::new(&v.sharing_level)]))
            .for_each(|c| {
                table.add_row(c);
            });

        table
    }
}

/// API response struct for token information.
/// Formats internal data for clients: combines entity/database into a path,
/// converts timestamps to strings, and translates permission levels to strings.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct APITokenInfo {
    pub short_token: String,
    pub scoped_database: Option<String>, // entity/database or None for unscoped
    pub permission_level: Option<String>, // "read-only" or "read-write" or None
    pub app_name: Option<String>,
    pub created_at: Option<String>,
    pub expires_at: Option<String>,
    pub revoked_at: Option<String>,
}

impl From<APITokenWithDatabase> for APITokenInfo {
    fn from(token: APITokenWithDatabase) -> Self {
        let scoped_database = match (token.entity_slug, token.database_slug) {
            (Some(entity), Some(db)) => Some(format!("{entity}/{db}")),
            _ => None,
        };

        let permission_level = token.query_permission_level.map(|level| {
            EntityDatabaseSharingLevel::try_from(level)
                .map(|l| l.to_str().to_string())
                .unwrap_or_else(|_| "unknown".to_string())
        });

        Self {
            short_token: token.short_token,
            scoped_database,
            permission_level,
            app_name: token.app_name,
            created_at: token.created_at.map(|dt| dt.to_string()),
            expires_at: token.expires_at.map(|dt| dt.to_string()),
            revoked_at: token.revoked_at.map(|dt| dt.to_string()),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TokenList {
    pub tokens: Vec<APITokenInfo>,
}

impl TabularFormatter for Vec<APITokenInfo> {
    fn to_table(&self) -> Table {
        let mut table = Table::new();
        table.set_titles(Row::new(vec![
            Cell::new("Short token"),
            Cell::new("Scope"),
            Cell::new("Permission"),
            Cell::new("App"),
            Cell::new("Created"),
            Cell::new("Expires"),
        ]));

        self.iter()
            .map(|v| {
                Row::new(vec![
                    Cell::new(&v.short_token),
                    Cell::new(v.scoped_database.as_deref().unwrap_or("(all databases)")),
                    Cell::new(v.permission_level.as_deref().unwrap_or("(full access)")),
                    Cell::new(v.app_name.as_deref().unwrap_or("")),
                    Cell::new(v.created_at.as_deref().unwrap_or("")),
                    Cell::new(v.expires_at.as_deref().unwrap_or("(never)")),
                ])
            })
            .for_each(|c| {
                table.add_row(c);
            });

        table
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ShortTokenPath {
    pub short_token: String,
}

// OAuth-related structs

/// Query parameters for OAuth authorization request (GET /oauth/authorize)
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OAuthAuthorizeRequest {
    pub response_type: String,         // Must be "code"
    pub redirect_uri: String,          // Where to redirect after authorization
    pub scope: String,                 // "read-only" or "read-write"
    pub state: Option<String>,         // Opaque value for CSRF protection
    pub code_challenge: String,        // PKCE: BASE64URL(SHA256(code_verifier))
    pub code_challenge_method: String, // Must be "S256"
    pub app_name: String,              // Display name for the app
}

/// Form data for OAuth authorization submit (POST /oauth/authorize)
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OAuthAuthorizeSubmit {
    pub database: String,         // Selected database as "entity/slug"
    pub permission_level: String, // "read-only" or "read-write"
    pub action: String,           // "authorize" or "deny"
    // These are preserved from the original request
    pub redirect_uri: String,
    pub state: Option<String>,
    pub code_challenge: String,
    pub app_name: String,
    pub requested_scope: String,
}

/// Request body for OAuth token exchange (POST /v1/oauth/token)
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OAuthTokenRequest {
    pub grant_type: String,    // Must be "authorization_code"
    pub code: String,          // The authorization code
    pub redirect_uri: String,  // Must match the original redirect_uri
    pub code_verifier: String, // PKCE: the original code_verifier
}

/// Response for OAuth token exchange
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OAuthTokenResponse {
    pub access_token: String,
    pub token_type: String,             // Always "Bearer"
    pub database: String,               // entity/database path
    pub query_permission_level: String, // "read-only" or "read-write"
    pub database_url: String,           // Full URL to the database API endpoint
}

/// Error response for OAuth
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OAuthErrorResponse {
    pub error: String,
    pub error_description: Option<String>,
}
