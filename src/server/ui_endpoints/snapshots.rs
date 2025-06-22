use crate::http::structs::EntityDatabasePath;
use crate::server::config::AybConfig;
use crate::server::ui_endpoints::auth::init_ayb_client;
use crate::server::ui_endpoints::templates::{error_snippet, render, success_snippet};
use actix_web::{get, post, web, HttpRequest, HttpResponse, Result};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct RestoreSnapshotRequest {
    snapshot_id: String,
}

#[get("/{entity}/{database}/snapshots")]
pub async fn database_snapshots(
    req: HttpRequest,
    path: web::Path<EntityDatabasePath>,
    ayb_config: web::Data<AybConfig>,
) -> Result<HttpResponse> {
    let entity_slug = &path.entity.to_lowercase();
    let database_slug = &path.database.to_lowercase();

    let client = init_ayb_client(&ayb_config, &req);

    match client.list_snapshots(entity_slug, database_slug).await {
        Ok(snapshot_list) => {
            let mut context = tera::Context::new();
            context.insert("snapshots", &snapshot_list.snapshots);

            let html = render("database_snapshots.html", &context);
            Ok(HttpResponse::Ok().content_type("text/html").body(html))
        }
        Err(err) => error_snippet("Error loading snapshots", &format!("{}", err)),
    }
}

#[post("/{entity}/{database}/restore_snapshot")]
pub async fn restore_snapshot(
    req: HttpRequest,
    path: web::Path<EntityDatabasePath>,
    form: web::Form<RestoreSnapshotRequest>,
    ayb_config: web::Data<AybConfig>,
) -> Result<HttpResponse> {
    let entity_slug = &path.entity.to_lowercase();
    let database_slug = &path.database.to_lowercase();
    let snapshot_id = &form.snapshot_id.trim();

    if snapshot_id.is_empty() {
        return error_snippet("Missing snapshot ID", "Please provide a valid snapshot ID.");
    }

    let client = init_ayb_client(&ayb_config, &req);

    match client
        .restore_snapshot(entity_slug, database_slug, snapshot_id)
        .await
    {
        Ok(_) => success_snippet(&format!(
            "Database successfully restored from snapshot '{}'.",
            snapshot_id
        )),
        Err(err) => error_snippet("Error restoring snapshot", &format!("{}", err)),
    }
}
