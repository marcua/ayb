use crate::ayb_db::db_interfaces::AybDb;
use crate::ayb_db::models::{InstantiatedEntity, Link, PartialEntity};
use crate::error::AybError;
use crate::http::structs::{EmptyResponse, EntityPath, ProfileLinkUpdate};
use crate::server::url_verification::is_verified_url;
use crate::server::utils::unwrap_authenticated_entity;
use crate::server::web_frontend::WebFrontendDetails;
use actix_web::{patch, web, HttpResponse};
use std::collections::HashMap;
use std::str::FromStr;
use url::Url;

#[patch("/entity/{entity}")]
pub async fn update_profile(
    path: web::Path<EntityPath>,
    profile: web::Json<HashMap<String, Option<String>>>,
    ayb_db: web::Data<Box<dyn AybDb>>,
    web_info: web::Data<Option<WebFrontendDetails>>,
    authenticated_entity: Option<web::ReqData<InstantiatedEntity>>,
) -> Result<HttpResponse, AybError> {
    let authenticated_entity = unwrap_authenticated_entity(&authenticated_entity)?;
    let entity_slug = &path.entity.to_lowercase();
    let profile = profile.into_inner();

    if entity_slug != &authenticated_entity.slug {
        return Ok(HttpResponse::Unauthorized().finish());
    }

    let links = if let Some(profile_links) = profile.get("links") {
        if let Some(profile_links) = profile_links {
            let profile_links: Vec<ProfileLinkUpdate> = serde_json::from_str(profile_links)?;

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

                Some(Some(links))
            } else {
                // If there is no known web frontend, we save links to the profile without verification.
                Some(Some(
                    profile_links
                        .into_iter()
                        .map(|l| Link {
                            url: l.url,
                            verified: false,
                        })
                        .collect(),
                ))
            }
        } else {
            Some(None)
        }
    } else {
        None
    };

    let mut partial = PartialEntity::new();
    partial.display_name = profile
        .get("display_name")
        .map(|v| v.as_ref().map(String::from));
    partial.description = profile
        .get("description")
        .map(|v| v.as_ref().map(String::from));
    partial.organization = profile
        .get("organization")
        .map(|v| v.as_ref().map(String::from));
    partial.location = profile
        .get("location")
        .map(|v| v.as_ref().map(String::from));
    partial.links = links;

    ayb_db
        .update_entity_by_id(authenticated_entity.id, &partial)
        .await?;

    Ok(HttpResponse::Ok().json(EmptyResponse {}))
}
