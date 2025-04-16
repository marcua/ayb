use crate::ayb_db::models::PublicSharingLevel;
use crate::http::structs::EntityPath;
use crate::server::config::AybConfig;
use crate::server::ui_endpoints::client::init_ayb_client;
use actix_web::{get, web, HttpRequest, HttpResponse, Result};

#[get("/{entity}/_fragments/create_database")]
pub async fn create_database(
    req: HttpRequest,
    path: web::Path<EntityPath>,
    ayb_config: web::Data<AybConfig>,
) -> Result<HttpResponse> {
    let entity_slug = &path.entity.to_lowercase();

    // Create the form HTML
    let form_html = format!(
        r##"<div class="block hover:bg-gray-50 uk-card">
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

            <div id="create-database-error"></div>

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
                const databaseSlug = document.getElementById('database-slug');
                const submitButton = document.getElementById('create-database-submit');
                submitButton.disabled = databaseSlug.value.trim() === '';

                databaseSlug.addEventListener('input', function() {{
                    submitButton.disabled = this.value.trim() === '';
                }});
            </script>
        </div>"##,
        entity = entity_slug,
        no_access = PublicSharingLevel::NoAccess.to_str(),
        fork = PublicSharingLevel::Fork.to_str(),
        read_only = PublicSharingLevel::ReadOnly.to_str()
    );

    Ok(HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(form_html))
}
