use crate::ayb_db::db_interfaces::AybDb;
use crate::ayb_db::models::{DBType, InstantiatedEntity, PublicSharingLevel};
use crate::error::AybError;
use crate::http::structs::{DatabaseDetails, EntityDatabasePath};
use crate::server::permissions::{
    can_discover_database, can_manage_database, highest_query_access_level,
};
use crate::server::utils::unwrap_authenticated_entity;
use actix_web::{get, web, HttpResponse, Result};

#[get("/{entity}/{database}/details")]
pub async fn database_details(
    path: web::Path<EntityDatabasePath>,
    ayb_db: web::Data<Box<dyn AybDb>>,
    authenticated_entity: Option<web::ReqData<InstantiatedEntity>>,
) -> Result<HttpResponse, AybError> {
    let authenticated_entity = unwrap_authenticated_entity(&authenticated_entity)?;
    let entity_slug = &path.entity.to_lowercase();
    let database_slug = &path.database;
    let database = ayb_db.get_database(entity_slug, database_slug).await?;

    // TODO(marcua): In the future, we might want to less
    // transparently 404 so that the presence of databases you can't
    // access isn't leaked.
    if can_discover_database(&authenticated_entity, &database, &ayb_db).await? {
        let can_manage = can_manage_database(&authenticated_entity, &database, &ayb_db).await?;
        let access_level =
            highest_query_access_level(&authenticated_entity, &database, &ayb_db).await?;

        let details = DatabaseDetails {
            entity_slug: entity_slug.to_string(),
            database_slug: database.slug,
            database_type: DBType::try_from(database.db_type).unwrap().to_str().into(),
            highest_query_access_level: access_level,
            can_manage_database: can_manage,
            public_sharing_level: PublicSharingLevel::try_from(database.public_sharing_level)
                .unwrap()
                .to_str()
                .into(),
        };

        Ok(HttpResponse::Ok().json(details))
    } else {
        Err(AybError::Other {
            message: format!(
                "Authenticated entity {} can't access database {}/{}",
                authenticated_entity.slug, entity_slug, database_slug
            ),
        })
    }
}
