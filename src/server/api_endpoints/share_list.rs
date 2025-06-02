use crate::ayb_db::db_interfaces::AybDb;
use crate::ayb_db::models::InstantiatedEntity;
use crate::error::AybError;
use crate::http::structs::{EntityDatabasePath, ShareList};
use crate::server::permissions::can_manage_database;
use crate::server::utils::unwrap_authenticated_entity;
use actix_web::{get, web, HttpResponse};

#[get("/{entity}/{database}/share_list")]
async fn share_list(
    path: web::Path<EntityDatabasePath>,
    ayb_db: web::Data<Box<dyn AybDb>>,
    authenticated_entity: Option<web::ReqData<InstantiatedEntity>>,
) -> Result<HttpResponse, AybError> {
    let entity_slug = &path.entity.to_lowercase();
    let database_slug = &path.database;
    
    let database = ayb_db
        .get_database(entity_slug, database_slug)
        .await?;
    
    let authenticated_entity = unwrap_authenticated_entity(&authenticated_entity)?;
    
    if !can_manage_database(&authenticated_entity, &database, &ayb_db).await? {
        return Err(AybError::Other {
            message: format!(
                "Authenticated entity {} can't list permissions for database {}/{}",
                authenticated_entity.slug, entity_slug, database_slug
            ),
        });
    }
    
    let sharing_entries = ayb_db
        .list_entity_database_permissions(&database)
        .await?;
    
    let share_list = ShareList {
        sharing_entries,
    };
    
    Ok(HttpResponse::Ok().json(share_list))
}
