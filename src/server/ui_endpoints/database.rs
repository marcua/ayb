use crate::http::structs::EntityDatabasePath;
use crate::server::config::AybConfig;
use crate::server::ui_endpoints::client::{init_ayb_client, logged_in_entity};
use actix_web::{get, web, HttpRequest, HttpResponse, Result};

#[get("/{entity}/{database}")]
pub async fn database(
    req: HttpRequest,
    path: web::Path<EntityDatabasePath>,
    ayb_config: web::Data<AybConfig>,
) -> Result<HttpResponse> {
    let entity_slug = &path.entity.to_lowercase();
    let database_slug = &path.database.to_lowercase();

    let client = init_ayb_client(&ayb_config, &req);

    // Get database details using the API client
    let database_response = match client.database_details(entity_slug, database_slug).await {
        Ok(response) => response,
        Err(_) => return Ok(HttpResponse::NotFound().body("Database not found")),
    };

    // Create breadcrumb navigation
    let breadcrumbs = format!(
        r#"<div class="breadcrumbs mb-4">
            <a href="/{}" class="hover:underline">{}</a> / 
            <span class="font-semibold">{}</span> ({})
        </div>"#,
        entity_slug, entity_slug, database_slug, database_response.database_type
    );

    // Create tabs based on permissions
    let tabs = format!(
        r#"<ul data-uk-tab class="mb-6">
            <li class="uk-active"><a class="px-4 pb-3 pt-2" href="">Query</a></li>
            {management_tabs}
        </ul>"#,
        management_tabs = if database_response.can_manage_database {
            r#"<li><a class="px-4 pb-3 pt-2" href="">Sharing</a></li>
               <li><a class="px-4 pb-3 pt-2" href="">Snapshots</a></li>"#
        } else {
            ""
        }
    );

    // Create query interface based on access level
    let query_interface = match database_response.highest_query_access_level {
        None => r#"<div class="uk-alert uk-alert-destructive" data-uk-alert="">
                <div class="uk-alert-title">You don't have query access to this database.</div>
                <p>You can request access from the database owner or fork a copy.</p>
            </div>"#
            .to_string(),
        Some(_) => format!(
            r##"<div class="query-interface">
                    <h3 class="text-lg font-medium mb-2">Database querying</h3>
                    <p class="text-muted-foreground mb-4">Select, add, and update data.</p>
                    <form
                      id="query-form"
                      class="mb-4"
                      action="/{entity}/{database}/query"
                      method="post"
                      hx-post="/{entity}/{database}/query"
                      hx-target="#query-results"
                      hx-target-400="#query-results"
                      hx-swap="innerHTML">
                        <div class="mb-2">
                            <textarea id="query" name="query" rows="5"
                                class="p-4 w-full border rounded focus:border-blue-500"
                                placeholder="Enter a SQL query, like 'SELECT * FROM your_table LIMIT 10'"></textarea>
                        </div>
                        <div>
                            <button type="submit" class="uk-btn uk-btn-primary" disabled id="run-query-btn">
                                Run query
                            </button>
                            <script>
                                // Form is submittable once a query exists.
                                document.addEventListener('DOMContentLoaded', function() {{
                                    const queryTextarea = document.getElementById('query');
                                    const runButton = document.getElementById('run-query-btn');
                                    runButton.disabled = queryTextarea.value.trim() === '';
                                    queryTextarea.addEventListener('input', function() {{
                                        runButton.disabled = this.value.trim() === '';
                                    }});
                                }});
                            </script>
                        </div>
                    </form>
                    <div id="query-results">
                    </div>
            </div>"##,
            entity = entity_slug,
            database = database_slug
        ),
    };

    // Create sharing interface (placeholder)
    let sharing_interface = format!(
        r##"<div class="sharing-interface">
                <h3 class="text-lg font-medium mb-2">Database sharing</h3>
                <p class="text-muted-foreground mb-4">Manage who can access this database and what permissions they have.</p>
                <p class="text-sm">Use the command line to manage sharing:</p>
                <pre class="bg-muted p-2 rounded mt-1 text-sm">ayb client share {entity_slug}/{database_slug} [entity] [sharing-level]</pre>
                <pre class="bg-muted p-2 rounded mt-1 text-sm">ayb client update_database --public_sharing_level [level] {entity_slug}/{database_slug}</pre>
        </div>"##,
        entity_slug = entity_slug,
        database_slug = database_slug
    );

    // Create snapshots interface (placeholder)
    let snapshots_interface = format!(
        r##"<div class="snapshots-interface">
                <h3 class="text-lg font-medium mb-2">Database snapshots</h3>
                <p class="text-muted-foreground mb-4">View and restore database snapshots.</p>
                <p class="text-sm">Use the command line to manage snapshots:</p>
                <pre class="bg-muted p-2 rounded mt-1 text-sm">ayb client list_snapshots {}/{}</pre>
                <pre class="bg-muted p-2 rounded mt-1 text-sm">ayb client restore_snapshot {}/{} [snapshot-id]</pre>
        </div>"##,
        entity_slug, database_slug, entity_slug, database_slug
    );

    // Create the tab switcher
    let tab_content = format!(
        r##"<ul class="uk-switcher mt-4">
            <li>{}</li>
            <li>{}</li>
            <li>{}</li>
        </ul>"##,
        query_interface, sharing_interface, snapshots_interface
    );

    // Combine all sections
    let content = format!(
        r#"
        <div class="max-w-screen-xl mx-auto">
            {}
            {}
            {}
        </div>
        "#,
        breadcrumbs, tabs, tab_content
    );

    let title = format!("{}/{}", entity_slug, database_slug);

    let current_entity = logged_in_entity(&req);

    Ok(HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(super::templates::base_content(
            &title,
            &content,
            current_entity.as_deref(),
        )))
}
