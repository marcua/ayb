use crate::ayb_db::db_interfaces::AybDb;
use crate::ayb_db::models::{InstantiatedEntity, Link, PartialEntity};
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

    let links = if let Some(profile_links) = profile.links {
        if let Some(web_info) = Option::as_ref(&**web_info) {
            // If there's a known web frontend, we verify the identity of the links.
            let mut links = vec![];
            for link in profile_links.into_iter() {
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
                profile_links
                    .into_iter()
                    .map(|l| Link {
                        url: l.url,
                        verified: false,
                    })
                    .collect(),
            )
        }
    } else {
        None
    };

    let mut partial = PartialEntity::new();
    partial.display_name = profile.display_name.map(Some);
    partial.description = profile.description.map(Some);
    partial.organization = profile.organization.map(Some);
    partial.location = profile.location.map(Some);
    partial.links = links.map(Some);

    ayb_db
        .update_entity_by_id(&partial, authenticated_entity.id)
        .await?;

    Ok(HttpResponse::Ok().json(EmptyResponse {}))
}
