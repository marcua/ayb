use crate::http::structs::EntityPath;
use crate::server::config::AybConfig;
use crate::server::ui_endpoints::auth::{authentication_details, init_ayb_client};
use crate::server::ui_endpoints::templates::{error_snippet, ok_response};
use actix_web::{get, post, web, HttpRequest, HttpResponse, Result};
use serde::Deserialize;
use std::collections::HashMap;

#[get("/{entity}")]
pub async fn entity_details(
    req: HttpRequest,
    path: web::Path<EntityPath>,
    ayb_config: web::Data<AybConfig>,
) -> Result<HttpResponse> {
    let entity_slug = &path.entity.to_lowercase();
    let client = init_ayb_client(&ayb_config, &req);

    // Get entity details using the API client
    let entity_response = match client.entity_details(entity_slug).await {
        Ok(response) => response,
        Err(_) => return Ok(HttpResponse::NotFound().body("Entity not found")),
    };

    let name = entity_response
        .profile
        .display_name
        .as_deref()
        .unwrap_or(&entity_response.slug);

    let mut context = tera::Context::new();
    context.insert("name", name);
    context.insert("entity", entity_slug);
    context.insert(
        "description",
        &entity_response.profile.description.unwrap_or_default(),
    );
    context.insert("organization", &entity_response.profile.organization);
    context.insert("location", &entity_response.profile.location);
    context.insert("links", &entity_response.profile.links);
    context.insert(
        "can_create_database",
        &entity_response.permissions.can_create_database,
    );
    context.insert("databases", &entity_response.databases);

    context.insert(
        "logged_in_entity",
        &authentication_details(&req).map(|details| details.entity),
    );

    ok_response("entity_details.html", &context)
}

#[derive(Deserialize)]
pub struct UpdateProfileRequest {
    display_name: Option<String>,
    description: Option<String>,
    organization: Option<String>,
    location: Option<String>,
    links: String, // JSON string of links
}

#[post("/{entity}/update_profile")]
pub async fn update_profile(
    req: HttpRequest,
    path: web::Path<EntityPath>,
    form: web::Form<UpdateProfileRequest>,
    ayb_config: web::Data<AybConfig>,
) -> Result<HttpResponse> {
    let entity_slug = &path.entity.to_lowercase();
    let client = init_ayb_client(&ayb_config, &req);

    // Check if the logged-in user is the same as the entity being updated
    let logged_in_entity = authentication_details(&req).map(|details| details.entity);
    if logged_in_entity.as_deref() != Some(entity_slug) {
        return error_snippet("Unauthorized", "You can only edit your own profile");
    }

    // Prepare the profile update data
    let mut profile_update = HashMap::new();
    profile_update.insert("display_name".to_string(), form.display_name.clone());
    profile_update.insert("description".to_string(), form.description.clone());
    profile_update.insert("organization".to_string(), form.organization.clone());
    profile_update.insert("location".to_string(), form.location.clone());
    profile_update.insert("links".to_string(), Some(form.links.clone()));

    // Update the profile using the API client
    match client.update_profile(entity_slug, &profile_update).await {
        Ok(_) => {
            // Fetch the updated entity details
            let entity_response = match client.entity_details(entity_slug).await {
                Ok(response) => response,
                Err(err) => {
                    return error_snippet("Error fetching updated profile", &format!("{err}"))
                }
            };

            let name = entity_response
                .profile
                .display_name
                .as_deref()
                .unwrap_or(&entity_response.slug);

            // Build context for the profile fragment
            let mut context = tera::Context::new();
            context.insert("name", name);
            context.insert("entity", entity_slug);
            context.insert(
                "description",
                &entity_response.profile.description.unwrap_or_default(),
            );
            context.insert("organization", &entity_response.profile.organization);
            context.insert("location", &entity_response.profile.location);
            context.insert("links", &entity_response.profile.links);
            context.insert(
                "logged_in_entity",
                &authentication_details(&req).map(|details| details.entity),
            );

            // Return the rendered profile fragment
            ok_response("profile_fragment.html", &context)
        }
        Err(err) => error_snippet("Error updating profile", &format!("{err}")),
    }
}
