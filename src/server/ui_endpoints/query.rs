use crate::http::structs::EntityDatabasePath;
use crate::server::config::AybConfig;
use crate::server::ui_endpoints::client::init_ayb_client;
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
    let query_result =
        match client.query(entity_slug, database_slug, query_text).await {
            Ok(result) => result,
            Err(err) => {
                let error_message = format!("Query error: {}", err);

                // Return error in the requested format
                return match format {
                "json" => Ok(HttpResponse::BadRequest().json(serde_json::json!({
                    "error": error_message
                }))),
                "csv" => Ok(HttpResponse::BadRequest()
                    .content_type("text/plain")
                    .body(format!("error,message\n\"{}\"", error_message.replace("\"", "\"\"")))),
                _ => Ok(HttpResponse::BadRequest().content_type("text/html").body(format!(
                    r#"<div class="error-message p-4 bg-red-50 border border-red-200 rounded">
                        <p class="text-red-700">{}</p>
                    </div>"#,
                    error_message
                ))),
            };
            }
        };

    // Return results in the requested format
    match format {
        "json" => Ok(HttpResponse::Ok().json(query_result)),
        "csv" => {
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
            // Format as HTML table
            let table_headers = query_result
                .fields
                .iter()
                .map(|field| format!("<th class=\"px-4 py-2 text-left\">{}</th>", field))
                .collect::<Vec<String>>()
                .join("");

            let table_rows = query_result
                .rows
                .iter()
                .map(|row| {
                    let cells = row
                        .iter()
                        .map(|cell| {
                            let display_value = match cell {
                                Some(value) => value,
                                None => "",
                            };
                            format!("<td class=\"px-4 py-2 border-t\">{}</td>", display_value)
                        })
                        .collect::<Vec<String>>()
                        .join("");
                    format!("<tr>{}</tr>", cells)
                })
                .collect::<Vec<String>>()
                .join("");

            let pagination_controls = if query_result.rows.len() >= 50 {
                r#"<div class="pagination mt-4 flex justify-between items-center">
                    <div>
                        <span class="text-sm text-gray-500">Showing 1-50 of results</span>
                    </div>
                    <div class="flex space-x-2">
                        <button disabled class="px-3 py-1 border rounded text-gray-400 bg-gray-100">Previous</button>
                        <button class="px-3 py-1 border rounded bg-white hover:bg-gray-50">Next</button>
                    </div>
                </div>"#.to_string()
            } else {
                format!(
                    r#"<div class="mt-4">
                    <span class="text-sm text-gray-500">Showing {} result{}</span>
                </div>"#,
                    query_result.rows.len(),
                    if query_result.rows.len() == 1 {
                        ""
                    } else {
                        "s"
                    }
                )
            };

            // Create a form for downloading in different formats
            let download_options = format!(
                r#"<div class="mt-4 flex space-x-2">
                    <form method="post" action="/{entity}/{database}/query" class="inline">
                        <input type="hidden" name="query" value="{}">
                        <input type="hidden" name="format" value="csv">
                        <button type="submit" class="px-3 py-1 border rounded bg-white hover:bg-gray-50 text-sm">Download CSV</button>
                    </form>
                    <form method="post" action="/{entity}/{database}/query" class="inline">
                        <input type="hidden" name="query" value="{}">
                        <input type="hidden" name="format" value="json">
                        <button type="submit" class="px-3 py-1 border rounded bg-white hover:bg-gray-50 text-sm">Download JSON</button>
                    </form>
                </div>"#,
                query_text,
                query_text,
                entity = entity_slug,
                database = database_slug
            );

            let html = format!(
                r#"<div class="query-results">
                    <div class="overflow-x-auto">
                        <table class="min-w-full bg-white">
                            <thead>
                                <tr class="bg-gray-100">{}</tr>
                            </thead>
                            <tbody>{}</tbody>
                        </table>
                    </div>
                    {}
                    {}
                </div>"#,
                table_headers, table_rows, pagination_controls, download_options
            );

            Ok(HttpResponse::Ok().content_type("text/html").body(html))
        }
    }
}
