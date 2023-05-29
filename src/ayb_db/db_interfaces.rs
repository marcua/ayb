use crate::ayb_db::models::{Database, Entity, InstantiatedDatabase, InstantiatedEntity};
use crate::error::AybError;
use async_trait::async_trait;
use dyn_clone::{clone_trait_object, DynClone};
use sqlx::{
    migrate,
    postgres::PgPoolOptions,
    sqlite::{SqliteConnectOptions, SqlitePoolOptions},
    Pool, Postgres, Sqlite,
};
use std::str::FromStr;

// AybDb is a trait for a database interface for storing `ayb`'s
// metadata. To support different databases (e.g., SQLite and
// Postgres) via `sqlx`, which requires static types for connection
// pools and query execution, the AybDb trait is implemented for each
// database, with shared coe provided by the `implement_ayb_db`
// macro. This is inspired by the `seafowl` project's implementation,
// the details of which can be found here:
// https://github.com/splitgraph/seafowl/blob/542159ebb42cada59cea6bd82fef4ab9e9724a94/src/repository/default.rs#L28
#[async_trait]
pub trait AybDb: DynClone + Send + Sync {
    async fn create_database(&self, database: &Database) -> Result<InstantiatedDatabase, AybError>;
    async fn create_entity(&self, entity: &Entity) -> Result<InstantiatedEntity, AybError>;
    async fn get_database(
        &self,
        entity_slug: &String,
        database_slug: &String,
    ) -> Result<InstantiatedDatabase, AybError>;
    async fn get_entity(&self, entity_slug: &String) -> Result<InstantiatedEntity, AybError>;
}

clone_trait_object!(AybDb);

#[macro_export]
macro_rules! implement_ayb_db {
    ($db_type: ident) => {
        #[async_trait]
        impl AybDb for $db_type {
            async fn create_database(
                &self, database: &Database,
            ) -> Result<InstantiatedDatabase, AybError> {
                let db: InstantiatedDatabase = sqlx::query_as(
                    r#"
                INSERT INTO database ( entity_id, slug, db_type )
                VALUES ( $1, $2, $3 )
                RETURNING id, entity_id, slug, db_type
                "#)
                .bind(database.entity_id)
                .bind(&database.slug)
                .bind(database.db_type)
                .fetch_one(&self.pool)
                .await
                .or_else(|err| match err {
                    // TODO(marcua): Figure out why `db_error.code() == "23505"`, which is less brittle and should work according to the sqlx docs, thinks it's receiving an `Option` for `code()`.
                    sqlx::Error::Database(db_error) if db_error.message() == "duplicate key value violates unique constraint \"database_entity_id_slug_key\"" => Err(AybError {
                        message: format!("Database already exists")
                    }),
                    _ => Err(AybError::from(err)),
                })?;

                Ok(db)
            }

            async fn create_entity(&self, entity: &Entity) -> Result<InstantiatedEntity, AybError> {
                let entity: InstantiatedEntity = sqlx::query_as(
                    r#"
                INSERT INTO entity ( slug, entity_type )
                VALUES ( $1, $2 )
                RETURNING id, slug, entity_type
                "#)
                .bind(&entity.slug)
                .bind(entity.entity_type)
                .fetch_one(&self.pool)
                .await
                .or_else(|err| match err {
                    // TODO(marcua): Figure out why `db_error.code() == "23505"`, which is less brittle and should work according to the sqlx docs, thinks it's receiving an `Option` for `code()`.
                    sqlx::Error::Database(db_error)
                        if db_error.message()
                        == "duplicate key value violates unique constraint \"entity_slug_key\"" =>
                    {
                        Err(AybError {
                            message: format!("Entity already exists"),
                        })
                    }
                    _ => Err(AybError::from(err)),
                })?;

                Ok(entity)
            }

            async fn get_database(
                &self,
                entity_slug: &String,
                database_slug: &String,
            ) -> Result<InstantiatedDatabase, AybError> {
                let db: InstantiatedDatabase = sqlx::query_as(
                    r#"
SELECT
    database.id,
    database.slug,
    database.entity_id,
    database.db_type
FROM database
JOIN entity on database.entity_id = entity.id
WHERE
    entity.slug = $1
    AND database.slug = $2
        "#)
                    .bind(entity_slug)
                    .bind(database_slug)
                    .fetch_one(&self.pool)
                    .await?;

                Ok(db)
            }

            async fn get_entity(
                &self,
                entity_slug: &String,
            ) -> Result<InstantiatedEntity, AybError> {
                let entity: InstantiatedEntity = sqlx::query_as(
                    r#"
SELECT
    id,
    slug,
    entity_type
FROM entity
WHERE slug = $1
        "#)
                    .bind(entity_slug)
                    .fetch_one(&self.pool)
                    .await
                    .or_else(|err| match err {
                        sqlx::Error::RowNotFound => Err(AybError {
                            message: format!("Entity not found: {:?}", entity_slug),
                        }),
                        _ => Err(AybError::from(err)),
                    })?;

                Ok(entity)
            }
        }
    }
}

#[derive(Clone)]
struct SqliteAybDb {
    pub pool: Pool<Sqlite>,
}

impl SqliteAybDb {
    pub async fn connect(url: String) -> SqliteAybDb {
        let connection_options = SqliteConnectOptions::from_str(&url)
            .expect("Unable to interpret SQLite connection uri")
            .create_if_missing(true);
        let pool = SqlitePoolOptions::new()
            .connect_with(connection_options)
            .await
            .expect("Unable to connect to database");
        migrate!("./migrations/sqlite")
            .run(&pool)
            .await
            .expect("Unable to run migrations");
        return Self { pool: pool };
    }
}

implement_ayb_db!(SqliteAybDb);

#[derive(Clone)]
struct PostgresAybDb {
    pub pool: Pool<Postgres>,
}

impl PostgresAybDb {
    pub async fn connect(url: String) -> PostgresAybDb {
        let pool = PgPoolOptions::new()
            .max_connections(20)
            .connect(&url)
            .await
            .expect("Unable to connect to database");
        migrate!("./migrations/postgres")
            .run(&pool)
            .await
            .expect("Unable to run migrations");
        return Self { pool: pool };
    }
}

implement_ayb_db!(PostgresAybDb);

pub async fn connect_to_ayb_db(url: String) -> Result<Box<dyn AybDb>, AybError> {
    if url.starts_with("sqlite") {
        Ok(Box::new(SqliteAybDb::connect(url).await))
    } else if url.starts_with("postgres") {
        Ok(Box::new(PostgresAybDb::connect(url).await))
    } else {
        Err(AybError {
            message: format!(
                "Database type for {} is not supported (currently only SQLite and PostgreSQL)",
                url
            ),
        })
    }
}
