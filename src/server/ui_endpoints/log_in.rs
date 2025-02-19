use super::templates::{base_auth, create_client};
use crate::server::config::AybConfig;
use actix_web::{get, post, web, HttpResponse, Result};

static CREATE_ACCOUNT: &str = r#"<a href="/register" class="text-sm">Create account</a>"#;

#[get("/log_in")]
pub async fn log_in() -> Result<HttpResponse> {
    let content = r#"
        <div class="bg-white rounded-lg shadow-sm p-6">
            <h1 class="text-2xl font-bold mb-6">Log in</h1>
            <form method="POST" class="space-y-4">
                <div>
                    <label class="uk-form-label">Username</label>
                    <input type="text" name="username" required 
                           class="uk-input">
                </div>
                <button type="submit" 
                        class="uk-btn uk-btn-primary w-full">
                    Log in
                </button>
            </form>
        </div>
    "#;

    Ok(HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(base_auth("Log in", CREATE_ACCOUNT, content)))
}

#[derive(serde::Deserialize)]
pub struct LoginForm {
    username: String,
}

#[post("/log_in")]
pub async fn log_in_submit(
    form: web::Form<LoginForm>,
    ayb_config: web::Data<AybConfig>,
) -> Result<HttpResponse> {
    let client = create_client(&ayb_config, None);

    match client.log_in(&form.username).await {
        Ok(_) => {
            let content = r#"
<div class="uk-alert" data-uk-alert>
  <div class="uk-alert-title">Check email</div>
  <p>
    Please check your email for a confirmation link.
  </p>
</div>
            "#;

            Ok(HttpResponse::Ok()
                .content_type("text/html; charset=utf-8")
                .body(base_auth("Check email", CREATE_ACCOUNT, content)))
        }
        Err(_) => {
            let content = r#"
<div class="uk-alert uk-alert-destructive" data-uk-alert>
  <div class="uk-alert-title">Unable to log in</div>
  <p>
    You were unable to log in. Please try again.
  </p>
</div>
            "#;

            Ok(HttpResponse::Ok()
                .content_type("text/html; charset=utf-8")
                .body(base_auth("Login error", CREATE_ACCOUNT, content)))
        }
    }
}
