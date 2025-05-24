use crate::ayb_db::models::PublicSharingLevel;
use crate::http::structs::EntityPath;
use crate::server::config::AybConfig;
use crate::server::ui_endpoints::auth::{authentication_details, init_ayb_client};
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
    let entity_response = match client.entity_details(entity_slug).await {
        Ok(response) => response,
        Err(_) => return Ok(HttpResponse::NotFound().body("Entity not found")),
    };

    let create_db_button = if entity_response.permissions.can_create_database {
        format!(
            r#"<button
                data-uk-toggle="target: #create-database-form"
                class="uk-btn {style} uk-btn-sm">
                <uk-icon icon="plus"></uk-icon> Create database
            </button>"#,
            style = if entity_response.databases.is_empty() {
                "uk-btn-primary"
            } else {
                "uk-btn-default"
            }
        )
    } else {
        String::new()
    };

    let create_db_form = if entity_response.permissions.can_create_database {
        format!(
            r##"<div id="create-database-form" hidden>
          <div class="block hover:bg-gray-50 uk-card">
            <h3 class="uk-h3 flex uk-card-header font-normal pb-0">Create a new database</h3>
            <div class="uk-card-body">
                <form
                  class="mt-4"
                  hx-post="/{entity}/create_database"
                  hx-target-400="#create-database-error"
                  hx-swap="innerHTML">
                    <div class="mb-4">
                        <label for="database-slug" class="block text-sm font-medium mb-1">Database name</label>
                        <input
                            type="text"
                            id="database-slug"
                            name="database_slug"
                            class="p-2 border rounded focus:border-blue-500"
                            placeholder="example.sqlite"
                            pattern="[A-Za-z0-9\-_\.]+"
                            title="Only letters, numbers, underscores, hyphens, and periods are allowed"
                            required>
                    </div>

                    <div class="mb-4">
                        <label class="block text-sm font-medium mb-1">Public sharing level</label>
                        <div class="uk-btn-group" data-uk-button-radio>
                            <button
                                type="button"
                                class="uk-btn uk-btn-default uk-active"
                                data-value="{no_access}"
                                onclick="setPublicSharingLevel(this, '{no_access}')">
                                Private
                            </button>
                            <button
                                type="button"
                                class="uk-btn uk-btn-default"
                                data-value="{fork}"
                                onclick="setPublicSharingLevel(this, '{fork}')">
                                Forkable
                            </button>
                            <button
                                type="button"
                                class="uk-btn uk-btn-default"
                                data-value="{read_only}"
                                onclick="setPublicSharingLevel(this, '{read_only}')">
                                Read-only
                            </button>
                        </div>
                        <input type="hidden" id="public-sharing-level" name="public_sharing_level" value="{no_access}">
                    </div>
                    <div class="mt-4">
                        <button type="submit" id="create-database-submit" class="uk-btn uk-btn-primary">
                            Create database
                        </button>
                    </div>
                </form>
            </div>

            <div id="create-database-error" class="mx-4 mb-4"></div>

            <script>
                function setPublicSharingLevel(button, value) {{
                    // Update the hidden input value
                    document.getElementById('public-sharing-level').value = value;

                    // Update button states
                    const buttons = button.parentElement.querySelectorAll('button');
                    buttons.forEach(btn => {{
                        btn.classList.remove('uk-active');
                    }});
                    button.classList.add('uk-active');
                }}

                // Form is submittable once a slug exists.
                document.addEventListener('DOMContentLoaded', function() {{
                    const databaseSlug = document.getElementById('database-slug');
                    const submitButton = document.getElementById('create-database-submit');
                    submitButton.disabled = databaseSlug.value.trim() === '';
                    databaseSlug.addEventListener('input', function() {{
                        submitButton.disabled = this.value.trim() === '';
                    }});
                }});
            </script>
          </div>
        </div>"##,
            entity = entity_slug,
            no_access = PublicSharingLevel::NoAccess.to_str(),
            fork = PublicSharingLevel::Fork.to_str(),
            read_only = PublicSharingLevel::ReadOnly.to_str()
        )
    } else {
        String::new()
    };

    let name = entity_response
        .profile
        .display_name
        .as_deref()
        .unwrap_or(&entity_response.slug);
    
    let mut context = tera::Context::new();
    context.insert("name", name);
    context.insert("entity", entity_slug);
    context.insert("description", &entity_response.profile.description.unwrap_or_default());
    
    // Format organization with icon if present
    let organization = entity_response
        .profile
        .organization
        .map_or_else(String::new, |org| 
            format!(r#"<div class="flex items-center"><uk-icon icon="building" class="mr-1"></uk-icon> {}</div>"#, org)
        );
    context.insert("organization", &organization);
    
    // Format location with icon if present
    let location = entity_response
        .profile
        .location
        .map_or_else(String::new, |loc| 
            format!(r#"<div class="flex items-center"><uk-icon icon="map-pin" class="mr-1"></uk-icon> {}</div>"#, loc)
        );
    context.insert("location", &location);
    
    // Format links
    let links = entity_response
        .profile
        .links
        .into_iter()
        .map(|link| 
            format!(r#"<div class="flex items-center"><uk-icon icon="link" class="mr-1"></uk-icon><a href="{}" rel="nofollow me">{}</a></div>"#, link.url, link.url)
        )
        .collect::<Vec<_>>()
        .join("\n");
    context.insert("links", &links);
    
    context.insert("create_db_button", &create_db_button);
    context.insert("create_db_form", &create_db_form);
    
    // Format database list
    let database_list = if entity_response.databases.is_empty() {
        r#"
            <div class="block uk-card">
                <h3 class="uk-h3 flex space-y-2 uk-card-header font-normal">No databases...yet!</h3>
                <p class="uk-card-body space-y-2">Let's fix that by creating your first database.</p>
            </div>"#.to_string()
    } else {
        entity_response
            .databases
            .into_iter()
            .map(|db| format!(
                r#"
                <a href="{}/{}" class="block hover:bg-gray-50 uk-card">
                    <h3 class="uk-h3 flex space-y-2 uk-card-header font-normal" style="align-items: baseline;"><uk-icon icon="database" class="mr-1"></uk-icon>{} <uk-icon icon="chevron-right"></uk-icon></h3>
                    <p class="text-muted-foreground uk-card-body space-y-2">Type: {}</p>
                </a>"#,
                entity_slug, db.slug, db.slug, db.database_type
            ))
            .collect::<Vec<_>>()
            .join("\n")
    };
    context.insert("database_list", &database_list);
    
    context.insert("logged_in_entity", &authentication_details(&req).map(|details| details.entity));

    Ok(HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(
            super::templates::TEMPLATES
                .render("entity_details.html", &context)
                .unwrap_or_else(|e| {
                    eprintln!("Template error: {}", e);
                    format!("Error rendering template: {}", e)
                }),
        ))
}
