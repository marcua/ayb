use crate::ayb_db::models::{EntityDatabaseSharingLevel, PublicSharingLevel};
use crate::http::structs::EntityDatabasePath;
use crate::server::config::AybConfig;
use crate::server::ui_endpoints::auth::init_ayb_client;
use actix_web::{post, web, HttpRequest, HttpResponse, Result};
use serde::Deserialize;
use std::str::FromStr;

#[derive(Deserialize)]
pub struct UpdatePublicSharingRequest {
    public_sharing_level: String,
}

#[derive(Deserialize)]
pub struct ShareWithEntityRequest {
    entity: String,
    sharing_level: String,
}

#[post("/{entity}/{database}/update_public_sharing")]
pub async fn update_public_sharing(
    req: HttpRequest,
    path: web::Path<EntityDatabasePath>,
    form: web::Form<UpdatePublicSharingRequest>,
    ayb_config: web::Data<AybConfig>,
) -> Result<HttpResponse> {
    let entity_slug = &path.entity.to_lowercase();
    let database_slug = &path.database.to_lowercase();

    let public_sharing_level = match PublicSharingLevel::from_str(&form.public_sharing_level) {
        Ok(level) => level,
        Err(_) => {
            return Ok(HttpResponse::BadRequest()
                .content_type("text/html")
                .body(format!(
                    r#"<div class="uk-alert uk-alert-destructive" data-uk-alert="">
                        <div class="uk-alert-title">Invalid sharing level</div>
                        <p>The sharing level '{}' is not valid.</p>
                    </div>"#,
                    form.public_sharing_level
                )));
        }
    };

    let client = init_ayb_client(&ayb_config, &req);

    match client
        .update_database(entity_slug, database_slug, &public_sharing_level)
        .await
    {
        Ok(_) => Ok(HttpResponse::Ok().content_type("text/html").body(
            r#"<div class="uk-alert uk-alert-success" data-uk-alert="">
                <div class="uk-alert-title">Success</div>
                <p>Public sharing level updated successfully.</p>
            </div>"#,
        )),
        Err(err) => {
            let error_message = format!("{}", err);
            Ok(HttpResponse::BadRequest()
                .content_type("text/html")
                .body(format!(
                    r#"<div class="uk-alert uk-alert-destructive" data-uk-alert="">
                        <div class="uk-alert-title">Error updating sharing level</div>
                        <p>{}</p>
                    </div>"#,
                    error_message
                )))
        }
    }
}

#[post("/{entity}/{database}/share")]
pub async fn share_with_entity(
    req: HttpRequest,
    path: web::Path<EntityDatabasePath>,
    form: web::Form<ShareWithEntityRequest>,
    ayb_config: web::Data<AybConfig>,
) -> Result<HttpResponse> {
    let entity_slug = &path.entity.to_lowercase();
    let database_slug = &path.database.to_lowercase();
    let target_entity = &form.entity.trim().to_lowercase();

    let sharing_level = match EntityDatabaseSharingLevel::from_str(&form.sharing_level) {
        Ok(level) => level,
        Err(_) => {
            return Ok(HttpResponse::BadRequest()
                .content_type("text/html")
                .body(format!(
                    r#"<div class="uk-alert uk-alert-destructive" data-uk-alert="">
                        <div class="uk-alert-title">Invalid sharing level</div>
                        <p>The sharing level '{}' is not valid.</p>
                    </div>"#,
                    form.sharing_level
                )));
        }
    };

    if target_entity.is_empty() {
        return Ok(HttpResponse::BadRequest().content_type("text/html").body(
            r#"<div class="uk-alert uk-alert-destructive" data-uk-alert="">
                <div class="uk-alert-title">Missing username</div>
                <p>Please enter a username to share with.</p>
            </div>"#,
        ));
    }

    let client = init_ayb_client(&ayb_config, &req);

    match client
        .share_database(entity_slug, database_slug, target_entity, &sharing_level)
        .await
    {
        Ok(_) => Ok(HttpResponse::Ok().content_type("text/html").body(format!(
            r#"<div class="uk-alert uk-alert-success" data-uk-alert="">
                    <div class="uk-alert-title">Success</div>
                    <p>Database access updated for user '{}'.</p>
                </div>"#,
            target_entity
        ))),
        Err(err) => {
            let error_message = format!("{}", err);
            Ok(HttpResponse::BadRequest()
                .content_type("text/html")
                .body(format!(
                    r#"<div class="uk-alert uk-alert-destructive" data-uk-alert="">
                        <div class="uk-alert-title">Error updating access</div>
                        <p>{}</p>
                    </div>"#,
                    error_message
                )))
        }
    }
}
