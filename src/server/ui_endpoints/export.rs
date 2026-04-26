use crate::http::structs::EntityDatabasePath;
use crate::server::config::AybConfig;
use crate::server::ui_endpoints::auth::{authentication_details, redirect_to_login};
use crate::server::web_frontend::local_base_url;
use actix_web::{get, web, HttpRequest, HttpResponse, Result};

#[get("/{entity}/{database}/export")]
pub async fn export(
    req: HttpRequest,
    path: web::Path<EntityDatabasePath>,
    ayb_config: web::Data<AybConfig>,
) -> Result<HttpResponse> {
    let token = match authentication_details(&req).map(|d| d.token) {
        Some(t) => t,
        None => return Ok(redirect_to_login(&req)),
    };

    let entity_slug = path.entity.to_lowercase();
    let database_slug = path.database.clone();
    let url = format!(
        "{}/v1/{entity_slug}/{database_slug}/export",
        local_base_url(&ayb_config)
    );

    let upstream = match reqwest::Client::new()
        .get(&url)
        .bearer_auth(token)
        .send()
        .await
    {
        Ok(resp) => resp,
        Err(err) => {
            return Ok(HttpResponse::InternalServerError()
                .body(format!("Error contacting export endpoint: {err}")))
        }
    };

    let status = upstream.status();
    if !status.is_success() {
        let message = upstream.text().await.unwrap_or_default();
        let actix_status = actix_web::http::StatusCode::from_u16(status.as_u16())
            .unwrap_or(actix_web::http::StatusCode::INTERNAL_SERVER_ERROR);
        return Ok(HttpResponse::build(actix_status)
            .content_type("text/plain")
            .body(message));
    }

    // Bridge reqwest's `chunk()`-based pull to Actix's streaming body
    // via futures::stream::unfold. We don't enable the `stream`
    // reqwest feature here because it pulls in conflicting wasm deps.
    let stream = futures_util::stream::unfold(Some(upstream), |state| async move {
        let mut upstream = state?;
        match upstream.chunk().await {
            Ok(Some(bytes)) => Some((Ok(bytes), Some(upstream))),
            Ok(None) => None,
            Err(err) => Some((Err(std::io::Error::other(err.to_string())), None)),
        }
    });

    Ok(HttpResponse::Ok()
        .content_type("application/octet-stream")
        .append_header((
            "Content-Disposition",
            format!("attachment; filename=\"{database_slug}\""),
        ))
        .streaming(stream))
}
