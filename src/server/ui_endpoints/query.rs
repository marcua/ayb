use crate::http::structs::EntityDatabasePath;
use crate::server::config::AybConfig;
use crate::server::ui_endpoints::auth::init_ayb_client;
use crate::server::ui_endpoints::templates::ok_response;
use actix_web::{post, web, HttpRequest, HttpResponse, Result};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct QueryRequest {
    query: String,
    format: Option<String>,
}

#[post("/{entity}/{database}/query")]
pub async fn query(
    req: HttpRequest,
    path: web::Path<EntityDatabasePath>,
    query_req: web::Form<QueryRequest>,
    ayb_config: web::Data<AybConfig>,
) -> Result<HttpResponse> {
    let entity_slug = &path.entity.to_lowercase();
    let database_slug = &path.database.to_lowercase();
    let query_text = &query_req.query;
    let format = query_req.format.as_deref().unwrap_or("html");

    let client = init_ayb_client(&ayb_config, &req);

    // Execute the query using the API client
    let query_result = match client.query(entity_slug, database_slug, query_text).await {
        Ok(result) => result,
        Err(err) => {
            let error_message = format!("{}", err);

            // Return error in the requested format
            return match format {
                "json" => Ok(HttpResponse::BadRequest().json(serde_json::json!({
                    "error": error_message
                }))),
                "csv" => Ok(HttpResponse::BadRequest()
                    .content_type("text/plain")
                    .body(format!(
                        "error,message\n\"{}\"",
                        error_message.replace("\"", "\"\"")
                    ))),
                _ => Ok(HttpResponse::BadRequest()
                    .content_type("text/html")
                    .body(format!(
                        r#"<div class="uk-alert uk-alert-destructive" data-uk-alert="">
                        <div class="uk-alert-title">Error running query</div>
                        <p>{}</p>
                    </div>"#,
                        error_message
                    ))),
            };
        }
    };

    // Return results in the requested format
    match format {
        "json" => {
            let json_content = serde_json::to_string_pretty(&query_result).unwrap_or_default();

            Ok(HttpResponse::Ok()
                .content_type("application/json")
                .append_header((
                    "Content-Disposition",
                    format!(
                        "attachment; filename=\"query-result-{}-{}.json\"",
                        entity_slug, database_slug
                    ),
                ))
                .body(json_content))
        }
        "csv" => {
            // TODO(marcua): replace with CSV serde the first time this hacky code poses a problem.
            let mut csv_content = query_result.fields.join(",") + "\n";

            for row in query_result.rows {
                let csv_row = row
                    .iter()
                    .map(|cell| match cell {
                        Some(value) => {
                            if value.contains(",") || value.contains("\"") || value.contains("\n") {
                                format!("\"{}\"", value.replace("\"", "\"\""))
                            } else {
                                value.to_string()
                            }
                        }
                        None => "".to_string(),
                    })
                    .collect::<Vec<String>>()
                    .join(",");
                csv_content.push_str(&(csv_row + "\n"));
            }

            Ok(HttpResponse::Ok()
                .content_type("text/csv")
                .append_header((
                    "Content-Disposition",
                    format!(
                        "attachment; filename=\"query-result-{}-{}.csv\"",
                        entity_slug, database_slug
                    ),
                ))
                .body(csv_content))
        }
        _ => {
            // Get total number of rows and limit display to 500
            let total_rows = query_result.rows.len();
            let display_limit = 500;
            let display_rows = if total_rows > display_limit {
                &query_result.rows[0..display_limit]
            } else {
                &query_result.rows
            };

            let mut context = tera::Context::new();
            context.insert("entity", entity_slug);
            context.insert("database", database_slug);
            context.insert("query", query_text);
            context.insert("fields", &query_result.fields);
            context.insert("rows", &query_result.rows);
            context.insert("display_rows", &display_rows);
            context.insert("total_rows", &total_rows);
            context.insert("display_limit", &display_limit);

            Ok(HttpResponse::Ok()
                .content_type("text/html")
                .body(ok_response("query_results.html", &context)?.into_body()))
        }
    }
}
