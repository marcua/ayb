use crate::ayb_db::models::EntityType;
use crate::server::config::AybConfig;
use crate::server::ui_endpoints::client::init_ayb_client;
use crate::server::ui_endpoints::templates::base_auth;
use actix_web::{get, post, web, HttpRequest, HttpResponse, Result};

static LOG_IN: &str = r#"<a href="/log_in" class="text-sm">Log in</a>"#;

#[get("/register")]
pub async fn register() -> Result<HttpResponse> {
    let content = r#"
        <div class="bg-white rounded-lg shadow-sm p-6">
            <h1 class="text-2xl font-bold mb-6">Create account</h1>
            <form method="POST" class="space-y-4">
                <div>
                    <label class="uk-form-label">Username</label>
                    <input type="text" name="username" required 
                           class="uk-input">
                </div>
                <div>
                    <label class="uk-form-label">Email</label>
                    <input type="email" name="email" required 
                           class="uk-input">
                </div>
                <button type="submit" 
                        class="uk-btn uk-btn-primary w-full">
                    Create account
                </button>
            </form>
        </div>
    "#;

    Ok(HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(base_auth("Register", LOG_IN, content, None)))
}

#[derive(serde::Deserialize)]
pub struct RegisterForm {
    username: String,
    email: String,
}

#[post("/register")]
pub async fn register_submit(
    req: HttpRequest,
    form: web::Form<RegisterForm>,
    ayb_config: web::Data<AybConfig>,
) -> Result<HttpResponse> {
    let client = init_ayb_client(&ayb_config, &req);

    match client
        .register(&form.username, &form.email, &EntityType::User)
        .await
    {
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
                .body(base_auth("Check email", LOG_IN, content, None)))
        }
        Err(_) => {
            let content = r#"
<div class="uk-alert uk-alert-destructive" data-uk-alert>
  <div class="uk-alert-title">Unable to log in</div>
  <p>
    Registration failed. Please try again.
  </p>
</div>
            "#;

            Ok(HttpResponse::Ok()
                .content_type("text/html; charset=utf-8")
                .body(base_auth("Account creation error", LOG_IN, content, None)))
        }
    }
}
