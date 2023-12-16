use crate::ayb_db::db_interfaces::AybDb;
use crate::ayb_db::models::InstantiatedEntity;
use crate::error::AybError;
use crate::http::permissions::can_query;
use crate::http::structs::{EntityDatabase, EntityPath, EntityQueryResponse};
use crate::http::utils::unwrap_authenticated_entity;
use actix_web::{get, web};

#[get("/v1/entity/{entity}")]
pub async fn entity_details(
    path: web::Path<EntityPath>,
    ayb_db: web::Data<Box<dyn AybDb>>,
    authenticated_entity: Option<web::ReqData<InstantiatedEntity>>,
) -> Result<web::Json<EntityQueryResponse>, AybError> {
    let authenticated_entity = unwrap_authenticated_entity(&authenticated_entity)?;
    let entity_slug = &path.entity;
    let desired_entity = ayb_db.get_entity_by_slug(entity_slug).await?;

    if desired_entity.is_none() {
        return Err(AybError {
            message: format!("Entity not found {:?}", entity_slug),
        });
    }

    let desired_entity = desired_entity.unwrap();
    let databases = ayb_db
        .list_databases_by_entity(&desired_entity)
        .await?
        .into_iter()
        .filter(|v| can_query(&authenticated_entity, v))
        .map(From::from)
        .collect::<Vec<EntityDatabase>>();

    Ok(web::Json(EntityQueryResponse {
        slug: entity_slug.to_string(),
        databases,
    }))
}
