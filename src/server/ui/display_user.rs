use crate::http::structs::EntityPath;
use crate::server::config::AybConfig;
use crate::server::utils::get_optional_header;
use actix_web::{get, web, HttpRequest, HttpResponse, Result};

#[get("/{entity}")]
pub async fn display_user(
    req: HttpRequest,
    path: web::Path<EntityPath>,
    ayb_config: web::Data<AybConfig>,
) -> Result<HttpResponse> {
    let entity_slug = &path.entity.to_lowercase();

    // TODO(marcua): Move "get cookie and create client" into a utils module.
    let mut client = super::templates::create_client(&ayb_config, None);

    // Get auth token from cookie if present
    if let Ok(Some(token)) = get_optional_header(&req, "Cookie") {
        if let Some(auth_token) = token
            .split(';')
            .find(|c| c.trim().starts_with("auth="))
            .map(|c| c.trim()[5..].to_string())
        {
            client.api_token = Some(auth_token);
        }
    }

    // Get entity details using the API client
    let entity_response = match client.entity_details(&entity_slug).await {
        Ok(response) => response,
        Err(_) => return Ok(HttpResponse::NotFound().body("Entity not found")),
    };

    let content = format!(
        r#"
        <div class="bg-white rounded-lg shadow-sm p-6 mb-6">
            <h1 class="text-2xl font-bold mb-2">{}</h1>
            {}
            {}
            {}
        </div>
        <div class="bg-white rounded-lg shadow-sm p-6">
            <h2 class="text-xl font-semibold mb-4">Databases</h2>
            <div class="grid gap-4">
                {}
            </div>
        </div>"#,
        entity_response
            .profile
            .display_name
            .as_deref()
            .unwrap_or(&entity_response.slug),
        entity_response
            .profile
            .description
            .map_or_else(String::new, |desc| format!(
                "<p class=\"text-gray-600 mb-4\">{}</p>",
                desc
            )),
        entity_response
            .profile
            .organization
            .map_or_else(String::new, |org| format!(
                "<p class=\"text-sm text-gray-500 mb-2\">🏢 {}</p>",
                org
            )),
        entity_response
            .profile
            .location
            .map_or_else(String::new, |loc| format!(
                "<p class=\"text-sm text-gray-500\">📍 {}</p>",
                loc
            )),
        entity_response
            .databases
            .into_iter()
            .map(|db| format!(
                r#"<a href="/d/{}/{}" class="block p-4 border rounded-lg hover:bg-gray-50">
                    <h3 class="font-medium">{}</h3>
                    <p class="text-sm text-gray-500">Type: {}</p>
                </a>"#,
                entity_slug, db.slug, db.slug, db.database_type
            ))
            .collect::<Vec<_>>()
            .join("\n")
    );

    Ok(HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(super::templates::base_content("User Profile", &content)))
}
