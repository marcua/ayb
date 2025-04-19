use crate::ayb_db::models::PublicSharingLevel;
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
    let entity_response = match client.entity_details(entity_slug).await {
        Ok(response) => response,
        Err(_) => return Ok(HttpResponse::NotFound().body("Entity not found")),
    };

    let create_db_form = format!(
        r##"<div id="create-database-form" hidden>
          <div class="block hover:bg-gray-50 uk-card">
            <h3 class="uk-h3 flex uk-card-header font-normal pb-0">Create a new database</h3>
            <div class="uk-card-body">
                <p class="text-muted-foreground">Create a new SQLite database for your entity.</p>

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
    );

    let name = entity_response
        .profile
        .display_name
        .as_deref()
        .unwrap_or(&entity_response.slug);
    // TODO(marcua): Only show database creation button/form if you're allowed to create one. Add that detail to entity_details endpoint.
    let content = format!(
        r###"
<div class="flex flex-col md:flex-row gap-4">
    <div class="w-full md:w-1/3 lg:w-1/4">
        <div class="uk-card">
            <div class="uk-card-header space-y-2">
                <h1 class="uk-h2">{name}</h1>
                <p class="text-muted-foreground">{description}</p>
            </div>
            <div class="uk-card-body space-y-2">
                <p class="text-muted-foreground">{organization}</p>
                <p class="text-muted-foreground">{location}</p>
                <div class="mt-3">
                    {links}
                </div>
            </div>
        </div>
    </div>
    <div class="w-full md:w-2/3 lg:w-3/4">
        <div class="uk-card-header space-y-2 pr-0 flex justify-between items-center">
            <h2 class="uk-h2">Databases</h2>
<button  type="button"></button>
            <button
                data-uk-toggle="target: #create-database-form"
                class="uk-btn {create_db_button_style} uk-btn-sm">
                <uk-icon icon="plus"></uk-icon> Create database
            </button>
        </div>
        <div class="uk-card-body space-y-2 pr-0">
            <hr class="uk-hr" />
            {create_db_form}
            {database_list}
       </div>
    </div>
</div>
"###,
        name = name,
        description = entity_response.profile.description.unwrap_or_default(),
        organization = entity_response
            .profile
            .organization
            .map_or_else(String::new, |org| format!(r#"<div class="flex items-center"><uk-icon icon="building" class="mr-1"></uk-icon> {}</div>"#, org)),
        location = entity_response
            .profile
            .location
            .map_or_else(String::new, |loc| format!(r#"<div class="flex items-center"><uk-icon icon="map-pin" class="mr-1"></uk-icon> {}</div>"#, loc)),

        links = entity_response
            .profile
            .links
            .into_iter()
            .map(|link| format!(r#"<div class="flex items-center"><uk-icon icon="link" class="mr-1"></uk-icon><a href="{}" rel="nofollow me">{}</a></div>"#, link.url, link.url))
            .collect::<Vec<_>>()
            .join("\n"),
        create_db_button_style = if entity_response.databases.is_empty() { "uk-btn-primary" } else { "uk-btn-default" },
        create_db_form = create_db_form,
        database_list = if entity_response.databases.is_empty() {
            format!(
                r#"
                <div class="block uk-card">
                    <h3 class="uk-h3 flex space-y-2 uk-card-header font-normal">No databases...yet!</h3>
                    <p class="uk-card-body space-y-2">Let's fix that by creating your first database.</p>
                </div>"#
            )
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
        }
    );

    Ok(HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(super::templates::base_content(name, &content)))
}
