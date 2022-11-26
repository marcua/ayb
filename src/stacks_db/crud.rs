use crate::stacks_db::models::{
    Database, DatabaseOwner, InstantiatedDatabase, InstantiatedDatabaseOwner,
};
use sqlx;
use sqlx::postgres::PgPool;

pub async fn create_database(
    database: &Database,
    pool: &PgPool,
) -> Result<InstantiatedDatabase, String> {
    let rec = sqlx::query_as!(
        InstantiatedDatabase,
        r#"
INSERT INTO database ( owner_id, slug, db_type )
VALUES ( $1, $2, $3 )
RETURNING id, owner_id, slug, db_type
        "#,
        database.owner_id,
        database.slug,
        database.db_type
    )
    .fetch_one(pool)
    .await
    .expect("Unable to create database");

    Ok(rec)
}

pub async fn create_owner(
    owner: &DatabaseOwner,
    pool: &PgPool,
) -> Result<InstantiatedDatabaseOwner, String> {
    let rec = sqlx::query_as!(
        InstantiatedDatabaseOwner,
        r#"
INSERT INTO database_owner ( slug )
VALUES ( $1 )
RETURNING id, slug
        "#,
        owner.slug
    )
    .fetch_one(pool)
    .await
    .expect("Unable to create owner");

    Ok(rec)
}

pub async fn get_database(
    owner_slug: &String,
    database_slug: &String,
    pool: &PgPool,
) -> Result<InstantiatedDatabase, String> {
    let rec = sqlx::query_as!(
        InstantiatedDatabase,
        r#"
SELECT
    database.id,
    database.slug,
    database.owner_id,
    database.db_type
FROM database
JOIN database_owner on database.owner_id = database_owner.id
WHERE
    database_owner.slug = $1
    AND database.slug = $2
        "#,
        owner_slug,
        database_slug
    )
    .fetch_one(pool)
    .await
    .expect("Unable to retrieve database");

    Ok(rec)
}

pub async fn get_owner(
    owner_slug: &String,
    pool: &PgPool,
) -> Result<InstantiatedDatabaseOwner, String> {
    let rec = sqlx::query_as!(
        InstantiatedDatabaseOwner,
        r#"
SELECT id, slug
FROM database_owner
WHERE slug = $1
        "#,
        owner_slug
    )
    .fetch_one(pool)
    .await
    .expect("Unable to retrieve owner");

    Ok(rec)
}
