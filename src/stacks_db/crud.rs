use crate::stacks_db::models::{Database};
use derive_more::{Display, Error};
use serde::{Deserialize, Serialize};
use sqlx;
use sqlx::postgres::{PgPool};


#[derive(Debug, Display, Error)]
#[display(fmt = "{}", id)]
#[derive(Serialize, Deserialize)]
pub struct DatabaseCreationResult {
    id: i32,
}

pub async fn create_database(database: &Database, pool: &PgPool) -> Result<DatabaseCreationResult, String> {
    let rec = sqlx::query_as!(
        DatabaseCreationResult,
        r#"
INSERT INTO database ( owner_id, slug, db_type )
VALUES ( $1, $2, $3 )
RETURNING id
        "#,
        database.owner_id,
        database.slug,
        database.db_type as i16
    )
        .fetch_one(pool)
        .await
        .expect("Unable to create database");

    Ok(rec)
}

