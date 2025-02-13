use super::templates::{base_auth, create_client};
use crate::server::config::AybConfig;
use actix_web::{get, web, HttpResponse, Result};

#[get("/confirm/{token}")]
pub async fn confirm_page(
    path: web::Path<String>,
    ayb_config: web::Data<AybConfig>,
) -> Result<HttpResponse> {
    let token = path.into_inner();
    let client = create_client(&ayb_config, None);

    match client.confirm(&token).await {
        Ok(api_token) => {
            let content = format!(
                r#"
        <div class="bg-white rounded-lg shadow-sm p-6">
            <h1 class="text-2xl font-bold mb-6">Success</h1>
            <p class="text-sm text-muted-foreground mb-6">Confirmation complete. You are now logged in.</p>
            <a href="/d/{}"
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
                    format!("auth={}; Path=/; HttpOnly", api_token.token),
                ))
                .body(base_auth("Success", "", &content)))
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
                .body(base_auth("Confirmation failed", "", content)))
        }
    }
}
