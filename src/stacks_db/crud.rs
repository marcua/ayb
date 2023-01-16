use crate::error::StacksError;
use crate::stacks_db::models::{Database, Entity, InstantiatedDatabase, InstantiatedEntity};
use sqlx;
use sqlx::postgres::PgPool;

pub async fn create_database(
    database: &Database,
    pool: &PgPool,
) -> Result<InstantiatedDatabase, StacksError> {
    let rec = sqlx::query_as!(
        InstantiatedDatabase,
        r#"
INSERT INTO database ( entity_id, slug, db_type )
VALUES ( $1, $2, $3 )
RETURNING id, entity_id, slug, db_type
        "#,
        database.entity_id,
        database.slug,
        database.db_type
    )
    .fetch_one(pool)
    .await
    .or_else(|err| match err {
        // TODO(marcua): Figure out why `db_error.code() == "23505"`, which is less brittle and should work according to the sqlx docs, thinks it's receiving an `Option` for `code()`.
        sqlx::Error::Database(db_error) if db_error.message() == "duplicate key value violates unique constraint \"database_entity_id_slug_key\"" => Err(StacksError {
            error_string: format!("Database already exists")
        }),
        _ => Err(StacksError::from(err)),
    })?;

    Ok(rec)
}

pub async fn create_entity(
    entity: &Entity,
    pool: &PgPool,
) -> Result<InstantiatedEntity, StacksError> {
    let rec = sqlx::query_as!(
        InstantiatedEntity,
        r#"
INSERT INTO entity ( slug, entity_type )
VALUES ( $1, $2 )
RETURNING id, slug, entity_type
        "#,
        entity.slug,
        entity.entity_type
    )
    .fetch_one(pool)
    .await
    .or_else(|err| match err {
        // TODO(marcua): Figure out why `db_error.code() == "23505"`, which is less brittle and should work according to the sqlx docs, thinks it's receiving an `Option` for `code()`.
        sqlx::Error::Database(db_error)
            if db_error.message()
                == "duplicate key value violates unique constraint \"entity_slug_key\"" =>
        {
            Err(StacksError {
                error_string: format!("Entity already exists"),
            })
        }
        _ => Err(StacksError::from(err)),
    })?;

    Ok(rec)
}

pub async fn get_database(
    entity_slug: &String,
    database_slug: &String,
    pool: &PgPool,
) -> Result<InstantiatedDatabase, StacksError> {
    let rec = sqlx::query_as!(
        InstantiatedDatabase,
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
        entity_slug,
        database_slug
    )
    .fetch_one(pool)
    .await?;

    Ok(rec)
}

pub async fn get_entity(
    entity_slug: &String,
    pool: &PgPool,
) -> Result<InstantiatedEntity, StacksError> {
    let rec = sqlx::query_as!(
        InstantiatedEntity,
        r#"
SELECT
    id,
    slug,
    entity_type
FROM entity
WHERE slug = $1
        "#,
        entity_slug
    )
    .fetch_one(pool)
    .await
    .or_else(|err| match err {
        sqlx::Error::RowNotFound => Err(StacksError {
            error_string: format!("Entity not found: {:?}", entity_slug),
        }),
        _ => Err(StacksError::from(err)),
    })?;

    Ok(rec)
}
