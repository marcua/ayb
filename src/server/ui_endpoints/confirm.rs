use crate::server::config::AybConfig;
use crate::server::ui_endpoints::client::init_ayb_client;
use crate::server::ui_endpoints::templates::base_auth;
use actix_web::{get, web, HttpRequest, HttpResponse, Result};

static LOG_IN: &str = r#"<a href="/log_in" class="text-sm">Log in</a>"#;

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
            let content = format!(
                r#"
        <div class="bg-white rounded-lg shadow-sm p-6">
            <h1 class="text-2xl font-bold mb-6">Success</h1>
            <p class="text-sm text-muted-foreground mb-6">Confirmation complete. You are now logged in.</p>
            <a href="/{}"
               class="uk-btn uk-btn-primary w-full">
                        Go to Your Profile
            </a>
        </div>
                "#,
                api_token.entity
            );

            Ok(HttpResponse::Ok()
                .content_type("text/html; charset=utf-8")
                .append_header((
                    "Set-Cookie",
                    format!(
                        "auth={}:{}; Path=/; HttpOnly; Secure; SameSite=Strict",
                        api_token.entity, api_token.token
                    ),
                ))
                .body(base_auth(
                    "Success",
                    "",
                    &content,
                    Some(format!("/{}", api_token.entity)),
                )))
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
                .body(base_auth("Confirmation failed", LOG_IN, content, None)))
        }
    }
}
