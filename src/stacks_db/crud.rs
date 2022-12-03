use crate::stacks_db::models::{Database, Entity, InstantiatedDatabase, InstantiatedEntity};
use sqlx;
use sqlx::postgres::PgPool;

pub async fn create_database(
    database: &Database,
    pool: &PgPool,
) -> Result<InstantiatedDatabase, String> {
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
    .expect("Unable to create database");

    Ok(rec)
}

pub async fn create_entity(entity: &Entity, pool: &PgPool) -> Result<InstantiatedEntity, String> {
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
    .expect("Unable to create entity");

    Ok(rec)
}

pub async fn get_database(
    entity_slug: &String,
    database_slug: &String,
    pool: &PgPool,
) -> Result<InstantiatedDatabase, String> {
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
    .await
    .expect("Unable to retrieve database");

    Ok(rec)
}

pub async fn get_entity(entity_slug: &String, pool: &PgPool) -> Result<InstantiatedEntity, String> {
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
    .expect("Unable to retrieve entity");

    Ok(rec)
}
