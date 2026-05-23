use crate::ayb_db::db_interfaces::AybDb;
use crate::ayb_db::models::{DBType, InstantiatedEntity, PublicSharingLevel};
use crate::error::AybError;
use crate::http::structs::{DatabaseDetails, EntityDatabasePath};
use crate::server::permissions::{
    can_discover_database, can_manage_database, highest_query_access_level,
    is_publicly_discoverable,
};
use actix_web::{get, web, HttpResponse, Result};

#[get(
    "/{entity}/{database}/details",
    wrap = "actix_web::middleware::from_fn(crate::server::server_runner::optional_entity_validator)"
)]
pub async fn database_details(
    path: web::Path<EntityDatabasePath>,
    ayb_db: web::Data<Box<dyn AybDb>>,
    authenticated_entity: Option<web::ReqData<InstantiatedEntity>>,
) -> Result<HttpResponse, AybError> {
    let authenticated_entity = authenticated_entity.map(|e| e.into_inner());
    let entity_slug = &path.entity.to_lowercase();
    let database_slug = &path.database;
    let database = ayb_db.get_database(entity_slug, database_slug).await?;

    // TODO(marcua): In the future, we might want to less
    // transparently 404 so that the presence of databases you can't
    // access isn't leaked.
    let discoverable = match authenticated_entity.as_ref() {
        Some(entity) => can_discover_database(entity, &database, &ayb_db).await?,
        None => is_publicly_discoverable(&database)?,
    };

    if discoverable {
        let (can_manage, access_level) = match authenticated_entity.as_ref() {
            Some(entity) => {
                let can_manage = can_manage_database(entity, &database, &ayb_db).await?;
                let access_level =
                    highest_query_access_level(entity, &database, None, &ayb_db).await?;
                (can_manage, access_level)
            }
            None => (false, None),
        };

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
        let message = match authenticated_entity.as_ref() {
            Some(entity) => format!(
                "Authenticated entity {} can't access database {}/{}",
                entity.slug, entity_slug, database_slug
            ),
            None => format!("Database {entity_slug}/{database_slug} is not accessible"),
        };
        Err(AybError::Unauthorized { message })
    }
}
