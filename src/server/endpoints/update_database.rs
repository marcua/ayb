use crate::ayb_db::db_interfaces::AybDb;
use crate::ayb_db::models::{InstantiatedEntity, PartialDatabase, PublicSharingLevel};
use std::str::FromStr;

use crate::error::AybError;
use crate::http::structs::{EmptyResponse, EntityDatabasePath};
use crate::server::permissions::can_manage_database;
use crate::server::utils::{get_optional_header, unwrap_authenticated_entity};
use actix_web::{patch, web, HttpRequest, HttpResponse};

#[patch("/v1/{entity}/{database}/update")]
async fn update_database(
    path: web::Path<EntityDatabasePath>,
    req: HttpRequest,
    ayb_db: web::Data<Box<dyn AybDb>>,
    authenticated_entity: Option<web::ReqData<InstantiatedEntity>>,
) -> Result<HttpResponse, AybError> {
    let entity_slug = &path.entity.to_lowercase();
    let database_slug = &path.database;
    let database = ayb_db.get_database(entity_slug, database_slug).await?;
    let authenticated_entity = unwrap_authenticated_entity(&authenticated_entity)?;
    if can_manage_database(&authenticated_entity, &database) {
        let public_sharing_level = get_optional_header(&req, "public-sharing-level")?;
        let mut partial_database = PartialDatabase {
            public_sharing_level: None,
        };
        if public_sharing_level.is_some() {
            partial_database.public_sharing_level =
                Some(PublicSharingLevel::from_str(&public_sharing_level.unwrap())? as i16);
        }
        ayb_db
            .update_database_by_id(database.id, &partial_database)
            .await?;
        Ok(HttpResponse::Ok().json(EmptyResponse {}))
    } else {
        Err(AybError::Other {
            message: format!(
                "Authenticated entity {} can't update database {}/{}",
                authenticated_entity.slug, entity_slug, database_slug
            ),
        })
    }
}
