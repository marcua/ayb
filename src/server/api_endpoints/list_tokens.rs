use crate::ayb_db::db_interfaces::AybDb;
use crate::ayb_db::models::InstantiatedEntity;
use crate::error::AybError;
use crate::http::structs::{APITokenInfo, TokenList};
use crate::server::utils::unwrap_authenticated_entity;
use actix_web::{get, web};

#[get("/tokens")]
async fn list_tokens(
    ayb_db: web::Data<Box<dyn AybDb>>,
    authenticated_entity: Option<web::ReqData<InstantiatedEntity>>,
) -> Result<web::Json<TokenList>, AybError> {
    let authenticated_entity = unwrap_authenticated_entity(&authenticated_entity)?;

    let tokens = ayb_db.list_api_tokens(&authenticated_entity).await?;
    let token_list: Vec<APITokenInfo> = tokens.into_iter().map(APITokenInfo::from).collect();

    Ok(web::Json(TokenList { tokens: token_list }))
}
