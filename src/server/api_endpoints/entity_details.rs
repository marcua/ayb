use crate::ayb_db::db_interfaces::AybDb;
use crate::ayb_db::models::InstantiatedEntity;
use crate::error::AybError;
use crate::http::structs::{
    EntityPath, EntityPermissions, EntityProfile, EntityProfileLink, EntityQueryResponse,
};
use crate::server::permissions::{
    can_create_database, can_discover_database, is_publicly_discoverable,
};
use actix_web::{get, web};

#[get("/entity/{entity}")]
pub async fn entity_details(
    path: web::Path<EntityPath>,
    ayb_db: web::Data<Box<dyn AybDb>>,
    authenticated_entity: Option<web::ReqData<InstantiatedEntity>>,
) -> Result<web::Json<EntityQueryResponse>, AybError> {
    let authenticated_entity = authenticated_entity.map(|e| e.into_inner());
    let entity_slug = &path.entity.to_lowercase();
    let desired_entity = ayb_db.get_entity_by_slug(entity_slug).await?;

    let mut databases = Vec::new();
    for database in ayb_db.list_databases_by_entity(&desired_entity).await? {
        let visible = match authenticated_entity.as_ref() {
            Some(entity) => can_discover_database(entity, &database, &ayb_db).await?,
            None => is_publicly_discoverable(&database)?,
        };
        if visible {
            databases.push(database.into());
        }
    }

    let links: Vec<EntityProfileLink> = desired_entity.links.clone().map_or_else(Vec::new, |l| {
        l.iter()
            .map(|l| EntityProfileLink {
                url: l.url.to_string(),
                verified: l.verified,
            })
            .collect()
    });

    let can_create = match authenticated_entity.as_ref() {
        Some(entity) => can_create_database(entity, &desired_entity),
        None => false,
    };

    Ok(web::Json(EntityQueryResponse {
        slug: entity_slug.to_string(),
        profile: EntityProfile {
            display_name: desired_entity.display_name,
            description: desired_entity.description,
            organization: desired_entity.organization,
            location: desired_entity.location,
            links,
        },
        databases,
        permissions: EntityPermissions {
            can_create_database: can_create,
        },
    }))
}
