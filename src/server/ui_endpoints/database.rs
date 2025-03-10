use crate::ayb_db::db_interfaces::AybDb;
use crate::ayb_db::models::{
    DBType, EntityDatabaseSharingLevel, InstantiatedDatabase, InstantiatedEntity,
    PublicSharingLevel,
};
use crate::error::AybError;
use crate::server::api_endpoints::query::execute_query;
use crate::server::permissions::{
    check_database_access, check_database_access_level, DatabaseAccessLevel,
};
use crate::server::ui_endpoints::templates::{base_content, render_template};
use crate::server::utils::unwrap_authenticated_entity;
use crate::server::web_frontend::WebFrontendDetails;
use actix_web::{get, web, HttpResponse, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum QueryMode {
    None,
    ForkOnly,
    ReadOnly,
    ReadWrite,
}

impl From<DatabaseAccessLevel> for QueryMode {
    fn from(access_level: DatabaseAccessLevel) -> Self {
        match access_level {
            DatabaseAccessLevel::NoAccess => QueryMode::None,
            DatabaseAccessLevel::ForkOnly => QueryMode::ForkOnly,
            DatabaseAccessLevel::ReadOnly => QueryMode::ReadOnly,
            DatabaseAccessLevel::ReadWrite => QueryMode::ReadWrite,
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct DatabasePageContext {
    entity_slug: String,
    entity_display_name: String,
    database_slug: String,
    database_name: String,
    database_type: String,
    query_mode: QueryMode,
    is_manager: bool,
    query_results: Option<QueryResults>,
    error_message: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
struct QueryResults {
    columns: Vec<String>,
    rows: Vec<Vec<String>>,
    total_rows: usize,
    current_page: usize,
    total_pages: usize,
    has_more_results: bool,
}

#[get("/{entity_slug}/{database_slug}")]
pub async fn database_page(
    path: web::Path<(String, String)>,
    query_params: web::Query<HashMap<String, String>>,
    ayb_db: web::Data<Box<dyn AybDb>>,
    web_frontend: web::Data<Option<WebFrontendDetails>>,
    authenticated_entity: Option<web::ReqData<InstantiatedEntity>>,
) -> Result<HttpResponse, AybError> {
    let (entity_slug, database_slug) = path.into_inner();

    // Get entity details
    let entity = match ayb_db.get_entity_by_slug(&entity_slug).await {
        Ok(entity) => entity,
        Err(_) => {
            return Ok(HttpResponse::NotFound().body(render_template(
                "Entity Not Found",
                &format!("Entity '{}' not found", entity_slug),
                None,
            )));
        }
    };

    // Get database details
    let database = match ayb_db
        .get_database_by_slug(&entity_slug, &database_slug)
        .await
    {
        Ok(db) => db,
        Err(_) => {
            return Ok(HttpResponse::NotFound().body(render_template(
                "Database Not Found",
                &format!(
                    "Database '{}' not found for entity '{}'",
                    database_slug, entity_slug
                ),
                None,
            )));
        }
    };

    // Check access level
    let authenticated_entity_ref = unwrap_authenticated_entity(&authenticated_entity);
    let access_level =
        check_database_access_level(authenticated_entity_ref, &database, &ayb_db).await?;

    let query_mode = QueryMode::from(access_level);
    let is_manager = match authenticated_entity_ref {
        Some(entity) => {
            if entity.id == database.entity_id {
                true // Owner has manager access
            } else {
                // Check if the authenticated entity has manager access
                match ayb_db
                    .get_entity_database_sharing_level(&entity.id, &database.id)
                    .await
                {
                    Ok(level) => level == EntityDatabaseSharingLevel::Manager,
                    Err(_) => false,
                }
            }
        }
        None => false,
    };

    // Process query if provided
    let mut query_results = None;
    let mut error_message = None;

    if let Some(query) = query_params.get("query") {
        if matches!(query_mode, QueryMode::ReadOnly | QueryMode::ReadWrite) {
            // Execute the query
            match execute_query(
                &entity_slug,
                &database_slug,
                query.to_string(),
                ayb_db.as_ref(),
                authenticated_entity_ref,
            )
            .await
            {
                Ok(result) => {
                    // Process the result
                    let rows_per_page = 50;
                    let page = query_params
                        .get("page")
                        .and_then(|p| p.parse::<usize>().ok())
                        .unwrap_or(1);

                    let total_rows = result.rows.len();
                    let total_pages = (total_rows + rows_per_page - 1) / rows_per_page;
                    let start_idx = (page - 1) * rows_per_page;
                    let end_idx = std::cmp::min(start_idx + rows_per_page, total_rows);

                    let has_more_results = total_rows > 2000;
                    let display_rows = if has_more_results {
                        result.rows[start_idx..std::cmp::min(end_idx, 2000)].to_vec()
                    } else {
                        result.rows[start_idx..end_idx].to_vec()
                    };

                    query_results = Some(QueryResults {
                        columns: result.columns,
                        rows: display_rows,
                        total_rows,
                        current_page: page,
                        total_pages,
                        has_more_results,
                    });
                }
                Err(err) => {
                    error_message = Some(format!("Query error: {}", err));
                }
            }
        } else if matches!(query_mode, QueryMode::ForkOnly) {
            error_message = Some("You have fork-only access to this database. You cannot run queries, but you can fork it to create your own copy.".to_string());
        } else {
            error_message = Some("You do not have permission to query this database.".to_string());
        }
    }

    // Prepare context for the template
    let db_type_str = match database.db_type {
        DBType::Sqlite => "SQLite",
        DBType::Duckdb => "DuckDB",
    };

    let context = DatabasePageContext {
        entity_slug: entity.slug,
        entity_display_name: entity.display_name,
        database_slug: database.slug,
        database_name: database.name,
        database_type: db_type_str.to_string(),
        query_mode,
        is_manager,
        query_results,
        error_message,
    };

    // Render the template
    let content = format!(
        r#"
        <div class="bg-white rounded-lg shadow-sm p-6">
            <div class="mb-4">
                <nav class="text-sm breadcrumbs">
                    <ul>
                        <li><a href="/{}">{}</a></li>
                        <li>{}</li>
                    </ul>
                </nav>
            </div>
            
            <div class="mb-6">
                <h1 class="text-2xl font-bold">{}</h1>
                <div class="text-sm text-gray-600">
                    <p>Database Type: {}</p>
                    <p>Owner: {}</p>
                </div>
            </div>
            
            {}
            
            {}
            
            {}
        </div>
        "#,
        context.entity_slug, context.entity_display_name, context.database_name,
        context.database_name,
        context.database_type,
        context.entity_display_name,
        // Query interface
        match context.query_mode {
            QueryMode::None => r#"<div class="alert alert-error mb-4">
                <p>You do not have access to this database.</p>
            </div>"#.to_string(),
            QueryMode::ForkOnly => r#"<div class="alert alert-warning mb-4">
                <p>You have fork-only access to this database. You cannot run queries, but you can fork it to create your own copy.</p>
                <button class="btn btn-sm btn-primary mt-2">Fork Database</button>
            </div>"#.to_string(),
            QueryMode::ReadOnly | QueryMode::ReadWrite => format!(
                r#"<div class="mb-6">
                    <h2 class="text-xl font-semibold mb-2">Query Database</h2>
                    <form method="GET" action="/{}/{}">
                        <div class="mb-4">
                            <textarea name="query" rows="5" class="w-full p-2 border rounded"
                                placeholder="Enter your SQL query here...">{}</textarea>
                        </div>
                        <div class="flex justify-between">
                            <button type="submit" class="btn btn-primary">Run Query</button>
                            {}
                        </div>
                    </form>
                </div>"#,
                context.entity_slug, context.database_slug,
                query_params.get("query").unwrap_or(&String::new()),
                if context.query_mode == QueryMode::ReadOnly {
                    r#"<div class="text-sm text-gray-600">
                        <p>You have read-only access to this database. Modification queries will fail.</p>
                    </div>"#
                } else {
                    ""
                }
            ),
        },
        // Error message
        if let Some(error) = &context.error_message {
            format!(r#"<div class="alert alert-error mb-4">
                <p>{}</p>
            </div>"#, error)
        } else {
            String::new()
        },
        // Query results
        if let Some(results) = &context.query_results {
            format!(
                r#"<div class="mb-6">
                    <h2 class="text-xl font-semibold mb-2">Query Results</h2>
                    {}
                    <div class="overflow-x-auto">
                        <table class="table table-zebra w-full">
                            <thead>
                                <tr>
                                    {}
                                </tr>
                            </thead>
                            <tbody>
                                {}
                            </tbody>
                        </table>
                    </div>
                    
                    <div class="flex justify-between items-center mt-4">
                        <div class="text-sm text-gray-600">
                            Showing page {} of {} ({} total rows)
                        </div>
                        <div class="join">
                            {}
                        </div>
                    </div>
                    
                    {}
                </div>"#,
                // Download options
                if results.total_rows > 0 {
                    format!(
                        r#"<div class="mb-4">
                            <a href="/{}/{}/download?query={}&format=csv" 
                               class="btn btn-sm btn-outline mr-2">Download CSV</a>
                            <a href="/{}/{}/download?query={}&format=json"
                               class="btn btn-sm btn-outline">Download JSON</a>
                        </div>"#,
                        context.entity_slug, context.database_slug, 
                        query_params.get("query").unwrap_or(&String::new()),
                        context.entity_slug, context.database_slug,
                        query_params.get("query").unwrap_or(&String::new())
                    )
                } else {
                    String::new()
                },
                // Table headers
                results.columns.iter()
                    .map(|col| format!("<th>{}</th>", col))
                    .collect::<Vec<_>>()
                    .join(""),
                // Table rows
                results.rows.iter()
                    .map(|row| {
                        format!(
                            "<tr>{}</tr>",
                            row.iter()
                                .map(|cell| format!("<td>{}</td>", cell))
                                .collect::<Vec<_>>()
                                .join("")
                        )
                    })
                    .collect::<Vec<_>>()
                    .join(""),
                // Pagination info
                results.current_page, results.total_pages, results.total_rows,
                // Pagination controls
                if results.total_pages > 1 {
                    let mut pagination = Vec::new();
                    
                    // Previous page button
                    if results.current_page > 1 {
                        pagination.push(format!(
                            r#"<a href="/{}/{}?query={}&page={}" class="join-item btn btn-sm">«</a>"#,
                            context.entity_slug, context.database_slug,
                            query_params.get("query").unwrap_or(&String::new()),
                            results.current_page - 1
                        ));
                    }
                    
                    // Page numbers
                    let start_page = std::cmp::max(1, results.current_page.saturating_sub(2));
                    let end_page = std::cmp::min(results.total_pages, start_page + 4);
                    
                    for page in start_page..=end_page {
                        if page == results.current_page {
                            pagination.push(format!(
                                r#"<a class="join-item btn btn-sm btn-active">{}</a>"#,
                                page
                            ));
                        } else {
                            pagination.push(format!(
                                r#"<a href="/{}/{}?query={}&page={}" class="join-item btn btn-sm">{}</a>"#,
                                context.entity_slug, context.database_slug,
                                query_params.get("query").unwrap_or(&String::new()),
                                page, page
                            ));
                        }
                    }
                    
                    // Next page button
                    if results.current_page < results.total_pages {
                        pagination.push(format!(
                            r#"<a href="/{}/{}?query={}&page={}" class="join-item btn btn-sm">»</a>"#,
                            context.entity_slug, context.database_slug,
                            query_params.get("query").unwrap_or(&String::new()),
                            results.current_page + 1
                        ));
                    }
                    
                    pagination.join("")
                } else {
                    String::new()
                },
                // Results limit warning
                if results.has_more_results {
                    r#"<div class="alert alert-info mt-4">
                        <p>Results limited to 2000 rows. Use the download buttons above to get the complete dataset.</p>
                    </div>"#.to_string()
                } else {
                    String::new()
                }
            )
        } else {
            String::new()
        }
    );

    Ok(HttpResponse::Ok().body(base_content(&title, &content, None)))
}

#[get("/{entity_slug}/{database_slug}/download")]
pub async fn download_query_results(
    path: web::Path<(String, String)>,
    query_params: web::Query<HashMap<String, String>>,
    ayb_db: web::Data<Box<dyn AybDb>>,
    authenticated_entity: Option<web::ReqData<InstantiatedEntity>>,
) -> Result<HttpResponse, AybError> {
    let (entity_slug, database_slug) = path.into_inner();

    // Check if query and format parameters are provided
    let query = match query_params.get("query") {
        Some(q) => q,
        None => return Ok(HttpResponse::BadRequest().body("Query parameter is required")),
    };

    let format = query_params
        .get("format")
        .unwrap_or(&"csv".to_string())
        .to_lowercase();
    if format != "csv" && format != "json" {
        return Ok(HttpResponse::BadRequest().body("Format must be 'csv' or 'json'"));
    }

    // Get database details
    let database = match ayb_db
        .get_database_by_slug(&entity_slug, &database_slug)
        .await
    {
        Ok(db) => db,
        Err(_) => {
            return Ok(HttpResponse::NotFound().body("Database not found"));
        }
    };

    // Check access level
    let authenticated_entity_ref = unwrap_authenticated_entity(&authenticated_entity);
    let access_level =
        check_database_access_level(authenticated_entity_ref, &database, &ayb_db).await?;

    if !matches!(
        access_level,
        DatabaseAccessLevel::ReadOnly | DatabaseAccessLevel::ReadWrite
    ) {
        return Ok(
            HttpResponse::Forbidden().body("You do not have permission to query this database")
        );
    }

    // Execute the query
    let result = execute_query(
        &entity_slug,
        &database_slug,
        query.to_string(),
        ayb_db.as_ref(),
        authenticated_entity_ref,
    )
    .await?;

    // Format the results
    let filename = format!("{}-{}-query-results.{}", entity_slug, database_slug, format);

    if format == "csv" {
        let mut csv_content = String::new();

        // Add header row
        csv_content.push_str(&result.columns.join(","));
        csv_content.push_str("\n");

        // Add data rows
        for row in result.rows {
            let escaped_row: Vec<String> = row
                .iter()
                .map(|cell| {
                    if cell.contains(',') || cell.contains('"') || cell.contains('\n') {
                        format!("\"{}\"", cell.replace("\"", "\"\""))
                    } else {
                        cell.clone()
                    }
                })
                .collect();
            csv_content.push_str(&escaped_row.join(","));
            csv_content.push_str("\n");
        }

        Ok(HttpResponse::Ok()
            .content_type("text/csv")
            .append_header((
                "Content-Disposition",
                format!("attachment; filename=\"{}\"", filename),
            ))
            .body(csv_content))
    } else {
        // JSON format
        let json_content = serde_json::to_string(&result).unwrap_or_else(|_| "{}".to_string());

        Ok(HttpResponse::Ok()
            .content_type("application/json")
            .append_header((
                "Content-Disposition",
                format!("attachment; filename=\"{}\"", filename),
            ))
            .body(json_content))
    }
}
