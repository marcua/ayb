use crate::ayb_db::models::{
    DBType, EntityType, InstantiatedDatabase as PersistedDatabase, InstantiatedDatabase,
    InstantiatedEntity as PersistedEntity,
};
use prettytable::{format, Cell, Row, Table};
use serde::{Deserialize, Serialize};
use url::Url;

#[derive(Clone, Serialize, Deserialize)]
pub struct AybConfigWeb {
    pub info_url: Url,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct AybConfigCors {
    pub origin: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct AybConfigAuthentication {
    pub fernet_key: String,
    pub token_expiration_seconds: u64,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct AybConfigEmail {
    pub from: String,
    pub reply_to: String,
    pub smtp_host: String,
    pub smtp_port: u16,
    pub smtp_username: String,
    pub smtp_password: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct AybConfigIsolation {
    pub nsjail_path: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct AybConfig {
    pub host: String,
    pub port: u16,
    pub database_url: String,
    pub data_path: String,
    pub e2e_testing: Option<bool>,
    pub authentication: AybConfigAuthentication,
    pub email: AybConfigEmail,
    pub web: Option<AybConfigWeb>,
    pub cors: AybConfigCors,
    pub isolation: Option<AybConfigIsolation>,
}

impl AybConfig {
    pub fn e2e_testing_on(&self) -> bool {
        self.e2e_testing.unwrap_or(false)
    }
}

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

#[derive(Serialize, Deserialize, Clone)]
pub struct EntityProfileLink {
    pub url: String,
    pub verified: bool,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct EntityProfile {
    pub display_name: Option<String>,
    pub description: Option<String>,
    pub workplace: Option<String>,
    pub links: Vec<EntityProfileLink>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct EntityQueryResponse {
    pub slug: String,
    pub profile: EntityProfile,
    pub databases: Vec<EntityDatabase>,
}

impl EntityQueryResponse {
    pub fn to_table(&self) -> Table {
        let mut table = Table::new();
        table.set_titles(Row::new(vec![
            Cell::new("Database slug"),
            Cell::new("Type"),
        ]));

        self.databases
            .iter()
            .map(|v| Row::new(vec![Cell::new(&v.slug), Cell::new(&v.database_type)]))
            .for_each(|c| {
                table.add_row(c);
            });

        table
    }

    pub fn generate_table(&self) -> Result<(), std::io::Error> {
        let mut table = self.to_table();
        table.set_format(*format::consts::FORMAT_NO_BORDER_LINE_SEPARATOR);
        table.print(&mut std::io::stdout())?;
        Ok(())
    }

    pub fn generate_csv(&self) -> Result<(), std::io::Error> {
        let table = self.to_table();
        table.to_csv(std::io::stdout())?;
        Ok(())
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
    pub token: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EmptyResponse {}
