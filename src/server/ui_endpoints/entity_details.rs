use crate::http::structs::EntityPath;
use crate::server::config::AybConfig;
use crate::server::ui_endpoints::client::init_ayb_client;
use actix_web::{get, web, HttpRequest, HttpResponse, Result};

#[get("/{entity}")]
pub async fn entity_details(
    req: HttpRequest,
    path: web::Path<EntityPath>,
    ayb_config: web::Data<AybConfig>,
) -> Result<HttpResponse> {
    let entity_slug = &path.entity.to_lowercase();

    let client = init_ayb_client(&ayb_config, &req);

    // Get entity details using the API client
    let entity_response = match client.entity_details(&entity_slug).await {
        Ok(response) => response,
        Err(_) => return Ok(HttpResponse::NotFound().body("Entity not found")),
    };

    let name = entity_response
        .profile
        .display_name
        .as_deref()
        .unwrap_or(&entity_response.slug);
    let content = format!(
        r#"
<div class="flex flex-col md:flex-row gap-4">
    <div class="w-full md:w-1/3 lg:w-1/4">
        <div class="uk-card">
            <div class="uk-card-header space-y-2">
                <h1 class="uk-h2">{}</h1>
                <p class="text-muted-foreground">{}</p>
            </div>
            <div class="uk-card-body space-y-2">
                <p class="text-muted-foreground">{}</p>
                <p class="text-muted-foreground">{}</p>
                <div class="mt-3">
                    {}
                </div>
            </div>
        </div>
    </div>
    <div class="w-full md:w-2/3 lg:w-3/4">
        <div class="uk-card-header space-y-2">
            <h2 class="uk-h2">Databases</h2>
        </div>
        <div class="uk-card-body space-y-2">
            {}
       </div>
    </div>
</div>
"#,
        name,
        entity_response.profile.description.unwrap_or_default(),
        // TODO(marcua): Actual icons
        entity_response
            .profile
            .organization
            .map_or_else(String::new, |org| format!("🏢 {}", org)),
        entity_response
            .profile
            .location
            .map_or_else(String::new, |loc| format!("📍 {}", loc)),
        
        entity_response
            .profile
            .links
            .into_iter()
            .map(|link| format!(r#"<a class="block" href="{}" target="_blank" rel="nofollow">{}</a>"#, link.url, link.url))
            .collect::<Vec<_>>()
            .join("\n"),
        entity_response
            .databases
            .into_iter()
            .map(|db| format!(
                r#"<hr class="uk-hr" />
                <a href="/d/{}/{}" class="block hover:bg-gray-50">
                    <h3 class="uk-h3 flex" style="align-items: baseline;">{} <uk-icon icon="chevron-right"></uk-icon></h3>
                    <p class="text-muted-foreground">Type: {}</p>
                </a>"#,
                entity_slug, db.slug, db.slug, db.database_type
            ))
            .collect::<Vec<_>>()
            .join("\n")
    );

    Ok(HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(super::templates::base_content(&name, &content)))
}
