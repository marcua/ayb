use crate::http::structs::EntityDatabasePath;
use crate::server::config::AybConfig;
use crate::server::ui_endpoints::client::init_ayb_client;
use actix_web::{post, web, HttpRequest, HttpResponse, Result};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct QueryRequest {
    query: String,
    format: Option<String>,
    page: Option<usize>,
    page_size: Option<usize>,
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
    let page = query_req.page.unwrap_or(1);
    let page_size = std::cmp::min(query_req.page_size.unwrap_or(50), 100);

    let client = init_ayb_client(&ayb_config, &req);

    // TODO(marcua): Using WITH queries: 1) determine queryset size
    // without pulling entire resultset, and 2) push pagination logic
    // into the DB.

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

            // Calculate pagination information
            let total_rows = query_result.rows.len();
            let start_index = (page - 1) * page_size;
            let end_index = std::cmp::min(start_index + page_size, total_rows);

            // Only display the current page of results
            let paginated_rows = if start_index < total_rows {
                &query_result.rows[start_index..end_index]
            } else {
                &[]
            };

            // Create table rows only for the current page
            let table_rows = paginated_rows
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

            let pagination_controls = if total_rows > page_size {
                let has_prev = page > 1;
                let has_next = end_index < total_rows;
                let prev_class = if has_prev {
                    "px-3 py-1 border rounded bg-white hover:bg-gray-50"
                } else {
                    "px-3 py-1 border rounded text-gray-400 bg-gray-100"
                };
                let next_class = if has_next {
                    "px-3 py-1 border rounded bg-white hover:bg-gray-50"
                } else {
                    "px-3 py-1 border rounded text-gray-400 bg-gray-100"
                };

                format!(
                    r###"<div class="pagination mt-4 px-4 flex justify-between items-center">
                        <div>
                            Showing {}-{} of {} results
                        </div>
                        <div class="flex space-x-2">
                            <button
                                hx-post="/{entity}/{database}/query"
                                hx-target="#query-results"
                                hx-swap="innerHTML"
                                hx-vals='{{"query": "{query}", "page": "{prev_page}", "page_size": "{page_size}"}}'
                                class="{prev_class}"
                                {prev_disabled}>
                                Previous
                            </button>
                            <button
                                hx-post="/{entity}/{database}/query"
                                hx-target="#query-results"
                                hx-swap="innerHTML"
                                hx-vals='{{"query": "{query}", "page": "{next_page}", "page_size": "{page_size}"}}'
                                class="{next_class}"
                                {next_disabled}>
                                Next
                            </button>
                        </div>
                    </div>"###,
                    start_index + 1,
                    end_index,
                    total_rows,
                    entity = entity_slug,
                    database = database_slug,
                    query = query_text,
                    prev_page = page - 1,
                    next_page = page + 1,
                    page_size = page_size,
                    prev_class = prev_class,
                    next_class = next_class,
                    prev_disabled = if has_prev { "" } else { "disabled" },
                    next_disabled = if has_next { "" } else { "disabled" }
                )
            } else if query_result.rows.is_empty() {
                r#"<div>
                    Query executed successfully. No results returned.
                </div>"#
                    .to_string()
            } else {
                format!(
                    r#"<div class="mt-4 px-4">
                        Showing {} result{}</span>
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
            let download_options = if query_result.rows.is_empty() {
                // Don't show download buttons when there are no results
                String::new()
            } else {
                format!(
                    r#"<div class="mt-4 flex space-x-2 px-4">
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
                )
            };

            let html = format!(
                r#"<div class="border rounded p-4 bg-gray-50">
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
