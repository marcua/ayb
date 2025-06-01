use crate::ayb_db::models::{
    DBType, EntityType, InstantiatedDatabase as PersistedDatabase, InstantiatedDatabase,
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
pub struct SnapshotList {
    pub snapshots: Vec<ListSnapshotResult>,
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
