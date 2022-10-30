use sqlx::any::{AnyPool};
use crate::stacks_db::models::{Database};

async fn create_database(database: &Database, pool: &AnyPool) -> Result<i64, String> {
    let rec = sqlx::query!(
        r#"
INSERT INTO database ( owner_id, slug, db_type )
VALUES ( $1, $2, $3 )
RETURNING id
        "#,
        database.owner_id,
        database.slug,
        database.db_type
    )
    .fetch_one(pool)
    .await;
    Ok(rec.id)
}

