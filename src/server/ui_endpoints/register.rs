use crate::ayb_db::models::EntityType;
use crate::server::config::AybConfig;
use crate::server::ui_endpoints::auth::init_ayb_client;
use crate::server::ui_endpoints::templates::{ok_response, render};
use actix_web::{get, post, web, HttpRequest, HttpResponse, Result};

#[get("/register")]
pub async fn register() -> Result<HttpResponse> {
    ok_response("register.html", &tera::Context::new())
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
        Ok(_) => ok_response("register_check_email.html", &tera::Context::new()),
        Err(_) => ok_response("register_error.html", &tera::Context::new()),
    }
}
