use crate::ayb_db::db_interfaces::AybDb;
use crate::ayb_db::models::{Entity, InstantiatedEntity, Link};
use crate::error::AybError;
use crate::http::structs::{EmptyResponse, EntityPath, ProfileUpdate};
use crate::http::url_verification::is_verified_url;
use crate::http::utils::unwrap_authenticated_entity;
use crate::http::web_frontend::WebFrontendDetails;
use actix_web::{patch, web, HttpResponse};
use std::str::FromStr;
use url::Url;

#[patch("/v1/entity/{entity}")]
pub async fn update_profile(
    path: web::Path<EntityPath>,
    profile: web::Json<ProfileUpdate>,
    ayb_db: web::Data<Box<dyn AybDb>>,
    web_info: web::Data<Option<WebFrontendDetails>>,
    authenticated_entity: Option<web::ReqData<InstantiatedEntity>>,
) -> Result<HttpResponse, AybError> {
    let authenticated_entity = unwrap_authenticated_entity(&authenticated_entity)?;
    let entity_slug = &path.entity;
    let profile = profile.into_inner();

    if entity_slug != &authenticated_entity.slug {
        return Ok(HttpResponse::Unauthorized().finish());
    }

    let instantiated_entity = ayb_db.get_entity_by_slug(entity_slug).await?;
    let links = if let Some(web_info) = Option::as_ref(&**web_info) {
        // If there's a known web frontend, we verify the identity of the links.
        let mut links = vec![];
        for link in profile.links.into_iter() {
            let url = Url::parse(&link.url)?;
            links.push(Link {
                url: link.url,
                verified: is_verified_url(
                    url,
                    Url::from_str(&web_info.profile(entity_slug))
                        .expect("invalid web frontend url"),
                )
                .await,
            })
        }

        Some(links)
    } else {
        // If there is no known web frontend, we save links to the profile without verification.
        Some(
            profile
                .links
                .into_iter()
                .map(|l| Link {
                    url: l.url,
                    verified: false,
                })
                .collect(),
        )
    };

    let entity = Entity {
        slug: instantiated_entity.slug,
        entity_type: instantiated_entity.entity_type,
        display_name: profile.display_name,
        description: profile.description,
        organization: profile.organization,
        location: profile.location,
        links,
    };

    ayb_db
        .update_entity_by_id(&entity, instantiated_entity.id)
        .await?;

    Ok(HttpResponse::Ok().json(EmptyResponse {}))
}
