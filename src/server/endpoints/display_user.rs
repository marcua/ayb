use crate::http::structs::EntityQueryResponse;
use actix_web::{get, web, HttpResponse, Result};
use crate::ayb_db::db_interfaces::AybDb;
use crate::server::config::AybConfig;

#[get("/d/{username}")]
pub async fn display_user(
    path: web::Path<String>,
    ayb_db: web::Data<Box<dyn AybDb>>,
    ayb_config: web::Data<AybConfig>,
) -> Result<HttpResponse> {
    let username = path.into_inner();
    
    // Get entity details
    let entity = match ayb_db.get_entity_by_slug(&username).await {
        Ok(entity) => entity,
        Err(_) => return Ok(HttpResponse::NotFound().body("User not found")),
    };

    let databases = ayb_db.get_databases_for_entity(entity.id).await
        .unwrap_or_default();

    let html = format!(r#"
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>{} - AYB</title>
    <link rel="stylesheet" href="https://unpkg.com/franken-ui@2.0.0-internal.41/dist/css/core.min.css"/>
    <link rel="stylesheet" href="https://unpkg.com/franken-ui@2.0.0-internal.41/dist/css/utilities.min.css"/>
</head>
<body class="bg-gray-50">
    <div class="max-w-4xl mx-auto p-6">
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
        </div>
    </div>
</body>
</html>
"#,
        // Title
        entity.display_name.as_deref().unwrap_or(&entity.slug),
        // Username
        entity.display_name.as_deref().unwrap_or(&entity.slug),
        // Description
        entity.description.map_or_else(String::new, |desc| 
            format!("<p class=\"text-gray-600 mb-4\">{}</p>", desc)),
        // Organization
        entity.organization.map_or_else(String::new, |org| 
            format!("<p class=\"text-sm text-gray-500 mb-2\">🏢 {}</p>", org)),
        // Location
        entity.location.map_or_else(String::new, |loc| 
            format!("<p class=\"text-sm text-gray-500\">📍 {}</p>", loc)),
        // Databases
        databases.into_iter()
            .map(|db| format!(r#"
                <a href="/d/{}/{}" class="block p-4 border rounded-lg hover:bg-gray-50">
                    <h3 class="font-medium">{}</h3>
                    <p class="text-sm text-gray-500">Type: {}</p>
                </a>
            "#, 
                username,
                db.slug,
                db.slug,
                db.db_type
            ))
            .collect::<Vec<_>>()
            .join("\n")
    );

    Ok(HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(html))
}
