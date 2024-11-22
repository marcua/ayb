use crate::ayb_db::db_interfaces::AybDb;
use crate::ayb_db::models::{
    EntityDatabasePermission, EntityDatabaseSharingLevel, InstantiatedEntity,
};
use std::str::FromStr;

use crate::error::AybError;
use crate::http::structs::EntityDatabasePath;
use crate::server::permissions::can_manage_database;
use crate::server::utils::{get_required_header, unwrap_authenticated_entity};
use actix_web::{post, web, HttpRequest, HttpResponse};

#[post("/v1/{entity}/{database}/share")]
async fn entity_database_permission(
    path: web::Path<EntityDatabasePath>,
    req: HttpRequest,
    ayb_db: web::Data<Box<dyn AybDb>>,
    authenticated_entity: Option<web::ReqData<InstantiatedEntity>>,
) -> Result<HttpResponse, AybError> {
    let entity_for_database_slug = &path.entity.to_lowercase();
    let database_slug = &path.database;
    let database = ayb_db
        .get_database(entity_for_database_slug, database_slug)
        .await?;
    let sharing_level =
        EntityDatabaseSharingLevel::from_str(&get_required_header(&req, "sharing-level")?)?;
    let entity_for_permission = ayb_db
        .get_entity_by_slug(&get_required_header(&req, "entity-for-permission")?)
        .await?;
    let authenticated_entity = unwrap_authenticated_entity(&authenticated_entity)?;
    if entity_for_permission.id == database.entity_id {
        Err(AybError::CantSetOwnerPermissions {
            message: format!(
                "{} owns {}/{}, so their permissions are set",
                entity_for_permission.slug, entity_for_database_slug, database_slug
            ),
        })
    } else if can_manage_database(&authenticated_entity, &database, &ayb_db).await? {
        if sharing_level == EntityDatabaseSharingLevel::NoAccess {
            ayb_db
                .delete_entity_database_permission(database.entity_id, database.id)
                .await?;
        } else {
            let permission = EntityDatabasePermission {
                entity_id: entity_for_permission.id,
                database_id: database.id,
                sharing_level: sharing_level as i16,
            };
            ayb_db
                .update_or_create_entity_database_permission(&permission)
                .await?;
        }

        Ok(HttpResponse::NoContent().into())
    } else {
        Err(AybError::Other {
            message: format!(
                "Authenticated entity {} can't set permissions for database {}/{}",
                authenticated_entity.slug, entity_for_database_slug, database_slug
            ),
        })
    }
}
