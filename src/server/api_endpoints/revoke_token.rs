use crate::ayb_db::db_interfaces::AybDb;
use crate::ayb_db::models::InstantiatedEntity;
use crate::error::AybError;
use crate::http::structs::{EmptyResponse, ShortTokenPath};
use crate::server::utils::unwrap_authenticated_entity;
use actix_web::{delete, web};

#[delete("/tokens/{short_token}")]
async fn revoke_token(
    path: web::Path<ShortTokenPath>,
    ayb_db: web::Data<Box<dyn AybDb>>,
    authenticated_entity: Option<web::ReqData<InstantiatedEntity>>,
) -> Result<web::Json<EmptyResponse>, AybError> {
    let authenticated_entity = unwrap_authenticated_entity(&authenticated_entity)?;

    ayb_db
        .revoke_api_token(&authenticated_entity, &path.short_token)
        .await?;

    Ok(web::Json(EmptyResponse {}))
}
