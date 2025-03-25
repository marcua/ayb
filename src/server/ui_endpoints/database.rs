use crate::http::structs::EntityDatabasePath;
use crate::server::config::AybConfig;
use crate::server::ui_endpoints::client::init_ayb_client;
use actix_web::{get, web, HttpRequest, HttpResponse, Result};

#[get("/{entity}/{database}")]
pub async fn database_details(
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
        r#"<div class="tabs mb-6">
            <div class="flex border-b">
                <button class="py-2 px-4 border-b-2 border-blue-500 font-medium text-blue-500">Query</button>
                {management_tabs}
            </div>
        </div>"#,
        management_tabs = if database_response.can_manage_database {
            r#"<button class="py-2 px-4 text-gray-500 hover:text-gray-700">Sharing</button>
               <button class="py-2 px-4 text-gray-500 hover:text-gray-700">Snapshots</button>"#
        } else {
            ""
        }
    );

    // Create query interface based on access level
    let query_interface = match database_response.highest_query_access_level {
        None => format!(
            r#"<div class="uk-card p-4 bg-red-50 border border-red-200 rounded mb-4">
                <p class="text-red-700">You don't have access to query this database.</p>
                <p class="mt-2">You can request access from the database owner or fork the database if public sharing allows it.</p>
            </div>"#
        ),
        Some(_) => format!(
            r##"<div class="query-interface mb-6">
                <form id="query-form" class="mb-4" action="/{entity}/{database}/query" method="post" hx-post="/{entity}/{database}/query" hx-target="#query-results" hx-swap="innerHTML">
                    <div class="mb-2">
                        <label for="query" class="block text-sm font-medium text-gray-700">SQL Query</label>
                        <textarea id="query" name="query" rows="5" 
                            class="mt-1 block w-full rounded-md border-gray-300 shadow-sm focus:border-blue-500 focus:ring-blue-500"
                            placeholder="SELECT * FROM your_table LIMIT 10"></textarea>
                    </div>
                    <div>
                        <button type="submit" class="inline-flex justify-center py-2 px-4 border border-transparent shadow-sm text-sm font-medium rounded-md text-white bg-blue-600 hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500">
                            Run Query
                        </button>
                    </div>
                </form>
                <div id="query-results" class="border rounded p-4 bg-gray-50">
                    <p class="text-gray-500">Query results will appear here</p>
                </div>
            </div>"##,
            entity = entity_slug,
            database = database_slug
        ),
    };

    // Combine all sections
    let content = format!(
        r#"
        <div class="max-w-screen-xl mx-auto">
            {}
            {}
            {}
            <div class="mt-4">
                <p class="text-sm text-gray-500">
                    For more advanced operations, use the command line:
                </p>
                <pre class="bg-gray-100 p-2 rounded mt-1 text-sm">ayb client query {} {}</pre>
            </div>
        </div>
        "#,
        breadcrumbs, tabs, query_interface, entity_slug, database_slug
    );

    let title = format!("{}/{}", entity_slug, database_slug);

    Ok(HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(super::templates::base_content(&title, &content)))
}
