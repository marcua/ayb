use crate::ayb_db::models::{
    APIToken, AuthenticationMethod, Database, Entity, InstantiatedAuthenticationMethod,
    InstantiatedDatabase, InstantiatedEntity,
};
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
    fn is_duplicate_constraint_error(&self, db_error: &dyn sqlx::error::DatabaseError) -> bool;
    async fn create_api_token(&self, api_token: &APIToken) -> Result<APIToken, AybError>;
    async fn create_authentication_method(
        &self,
        method: &AuthenticationMethod,
    ) -> Result<InstantiatedAuthenticationMethod, AybError>;
    async fn create_database(&self, database: &Database) -> Result<InstantiatedDatabase, AybError>;
    async fn get_or_create_entity(&self, entity: &Entity) -> Result<InstantiatedEntity, AybError>;
    async fn get_api_token(&self, short_token: &String) -> Result<APIToken, AybError>;
    async fn get_database(
        &self,
        entity_slug: &String,
        database_slug: &String,
    ) -> Result<InstantiatedDatabase, AybError>;
    async fn get_entity_by_slug(
        &self,
        entity_slug: &String,
    ) -> Result<InstantiatedEntity, AybError>;
    async fn get_entity_by_id(&self, entity_id: i32) -> Result<InstantiatedEntity, AybError>;
    async fn list_authentication_methods(
        &self,
        entity: &InstantiatedEntity,
    ) -> Result<Vec<InstantiatedAuthenticationMethod>, AybError>;
}

clone_trait_object!(AybDb);

#[macro_export]
macro_rules! implement_ayb_db {
    ($db_type: ident) => {
        #[async_trait]
        impl AybDb for $db_type {
            fn is_duplicate_constraint_error(
                &self,
                db_error: &dyn sqlx::error::DatabaseError,
            ) -> bool {
                match db_error.code() {
                    Some(code) => code.to_string() == $db_type::DUPLICATE_CONSTRAINT_ERROR_CODE,
                    None => false,
                }
            }

            async fn create_api_token(&self, api_token: &APIToken) -> Result<APIToken, AybError> {
                let returned_token: APIToken = sqlx::query_as(
                    r#"
                INSERT INTO api_token ( entity_id, short_token, hash, status )
                VALUES ( $1, $2, $3, $4 )
RETURNING entity_id, short_token, hash, status
                "#,
                )
                    .bind(api_token.entity_id)
                    .bind(&api_token.short_token)
                    .bind(&api_token.hash)
                    .bind(api_token.status)
                    .fetch_one(&self.pool)
                    .await?;

                Ok(returned_token)
            }

            async fn create_authentication_method(
                &self,
                method: &AuthenticationMethod,
            ) -> Result<InstantiatedAuthenticationMethod, AybError> {
                let instantiated_method: InstantiatedAuthenticationMethod = sqlx::query_as(
                    r#"
                INSERT INTO authentication_method ( entity_id, method_type, status, email_address )
                VALUES ( $1, $2, $3, $4 )
                RETURNING id, entity_id, method_type, status, email_address
                "#,
                )
                .bind(method.entity_id)
                .bind(method.method_type)
                .bind(method.status)
                .bind(&method.email_address)
                .fetch_one(&self.pool)
                .await?;

                Ok(instantiated_method)
            }

            async fn create_database(
                &self,
                database: &Database,
            ) -> Result<InstantiatedDatabase, AybError> {
                let db: InstantiatedDatabase = sqlx::query_as(
                    r#"
                INSERT INTO database ( entity_id, slug, db_type )
                VALUES ( $1, $2, $3 )
                RETURNING id, entity_id, slug, db_type
                "#,
                )
                .bind(database.entity_id)
                .bind(&database.slug)
                .bind(database.db_type)
                .fetch_one(&self.pool)
                .await
                .or_else(|err| match err {
                    sqlx::Error::Database(db_error)
                        if self.is_duplicate_constraint_error(&*db_error) =>
                    {
                        Err(AybError {
                            message: format!("Database already exists"),
                        })
                    }
                    _ => Err(AybError::from(err)),
                })?;

                Ok(db)
            }

            async fn get_api_token(
                &self,
                short_token: &String,
            ) -> Result<APIToken, AybError> {
                let api_token: APIToken = sqlx::query_as(
                    r#"
SELECT
    short_token,
    entity_id,
    hash,
    status
FROM api_token
WHERE short_token = $1
        "#,
                )
                .bind(short_token)
                .fetch_one(&self.pool)
                .await
                .or_else(|err| match err {
                    sqlx::Error::RowNotFound => Err(AybError {
                        message: format!("API Token not found: {:?}", short_token),
                    }),
                    _ => Err(AybError::from(err)),
                })?;

                Ok(api_token)
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
        "#,
                )
                .bind(entity_slug)
                .bind(database_slug)
                .fetch_one(&self.pool)
                .await?;

                Ok(db)
            }

            async fn get_entity_by_slug(
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
        "#,
                )
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

            async fn get_entity_by_id(
                &self,
                entity_id: i32,
            ) -> Result<InstantiatedEntity, AybError> {
                let entity: InstantiatedEntity = sqlx::query_as(
                    r#"
SELECT
    id,
    slug,
    entity_type
FROM entity
WHERE id = $1
        "#,
                )
                .bind(entity_id)
                .fetch_one(&self.pool)
                .await
                .or_else(|err| match err {
                    sqlx::Error::RowNotFound => Err(AybError {
                        message: format!("Entity not found: {:?}", entity_id),
                    }),
                    _ => Err(AybError::from(err)),
                })?;

                Ok(entity)
            }

            async fn get_or_create_entity(&self, entity: &Entity) -> Result<InstantiatedEntity, AybError> {
                // Get or create logic inspired by https://stackoverflow.com/a/66337293
                let mut tx = self.pool.begin().await?;
                sqlx::query(
                    r#"
INSERT INTO entity ( slug, entity_type )
VALUES ( $1, $2 )
ON CONFLICT (slug) DO UPDATE
    SET entity_type = $2
    WHERE false;
                "#,
                )
                .bind(&entity.slug)
                .bind(entity.entity_type)
                .execute(&mut tx)
                .await?;
                let entity: InstantiatedEntity = sqlx::query_as(
                    r#"
SELECT id, slug, entity_type
FROM entity
WHERE slug = $1;
                "#,
                )
                .bind(&entity.slug)
                .fetch_one(&mut tx)
                .await?;
                tx.commit().await?;
                Ok(entity)
            }

            async fn list_authentication_methods(
                &self,
                entity: &InstantiatedEntity,
            ) -> Result<Vec<InstantiatedAuthenticationMethod>, AybError> {
                let methods: Vec<InstantiatedAuthenticationMethod> = sqlx::query_as(
                    r#"
SELECT
    id,
    entity_id,
    method_type,
    status,
    email_address
FROM authentication_method
WHERE entity_id = $1
        "#,
                )
                .bind(entity.id)
                .fetch_all(&self.pool)
                .await?;

                Ok(methods)
            }
        }
    };
}

#[derive(Clone)]
struct SqliteAybDb {
    pub pool: Pool<Sqlite>,
}

impl SqliteAybDb {
    pub const DUPLICATE_CONSTRAINT_ERROR_CODE: &'static str = "2067";

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
    pub const DUPLICATE_CONSTRAINT_ERROR_CODE: &'static str = "23505";

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
