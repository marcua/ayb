use crate::server::config::AybConfig;
use crate::server::ui_endpoints::client::init_ayb_client;
use crate::server::ui_endpoints::templates::base_auth;
use actix_web::{get, web, HttpRequest, HttpResponse, Result};

#[get("/confirm/{token}")]
pub async fn confirm(
    req: HttpRequest,
    path: web::Path<String>,
    ayb_config: web::Data<AybConfig>,
) -> Result<HttpResponse> {
    let token = path.into_inner();
    let client = init_ayb_client(&ayb_config, &req);

    match client.confirm(&token).await {
        Ok(api_token) => {
            Ok(HttpResponse::Found()
                .append_header((
                    "Set-Cookie",
                    format!(
                        "auth={}; Path=/; HttpOnly; Secure; SameSite=Strict",
                        api_token.token
                    ),
                ))
                .append_header(("Location", format!("/{}", api_token.entity)))
                .finish())
        }
        Err(_) => {
            let content = r#"
<div class="uk-alert uk-alert-destructive" data-uk-alert>
  <div class="uk-alert-title">Unable to log in</div>
  <p>
    Invalid or expired confirmation link. Please try again.
  </p>
</div>
            "#;

            Ok(HttpResponse::Ok()
                .content_type("text/html; charset=utf-8")
                .body(base_auth("Confirmation failed", "", content, None)))
        }
    }
}
