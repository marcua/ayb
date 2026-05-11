use crate::ayb_db::db_interfaces::AybDb;
use crate::ayb_db::models::{InstantiatedEntity, Link, PartialEntity};
use crate::error::AybError;
use crate::http::structs::{EmptyResponse, EntityPath, ProfileLinkUpdate};
use crate::server::config::{public_base_url, AybConfig};
use crate::server::url_verification::is_verified_url;
use crate::server::utils::unwrap_authenticated_entity;
use actix_web::{patch, web, HttpResponse};
use std::collections::HashMap;
use std::str::FromStr;
use url::Url;

#[patch(
    "/entity/{entity}",
    wrap = "actix_web_httpauth::middleware::HttpAuthentication::bearer(crate::server::server_runner::entity_validator)"
)]
pub async fn update_profile(
    path: web::Path<EntityPath>,
    profile: web::Json<HashMap<String, Option<String>>>,
    ayb_db: web::Data<Box<dyn AybDb>>,
    ayb_config: web::Data<AybConfig>,
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
            let expected_profile_url =
                Url::from_str(&format!("{}/{}", public_base_url(&ayb_config), entity_slug))
                    .expect("invalid public_url in config");

            let mut links = vec![];
            for link in profile_links.into_iter() {
                let url = Url::parse(&link.url)?;
                links.push(Link {
                    url: link.url,
                    verified: is_verified_url(url, expected_profile_url.clone()).await,
                })
            }

            Some(Some(links))
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

    // Check if there are any fields to update
    if !partial.has_updates() {
        return Err(AybError::EmptyUpdateError {
            message: "No fields provided to update. Please specify at least one field to update."
                .to_string(),
        });
    }

    ayb_db
        .update_entity_by_id(authenticated_entity.id, &partial)
        .await?;

    Ok(HttpResponse::Ok().json(EmptyResponse {}))
}
