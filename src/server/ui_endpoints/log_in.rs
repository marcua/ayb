use crate::server::config::AybConfig;
use crate::server::ui_endpoints::auth::init_ayb_client;
use crate::server::ui_endpoints::templates::render;
use actix_web::{get, post, web, HttpRequest, HttpResponse, Result};

#[get("/log_in")]
pub async fn log_in() -> Result<HttpResponse> {
    Ok(HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(render("log_in.html", &tera::Context::new())))
}

#[derive(serde::Deserialize)]
pub struct LoginForm {
    username: String,
}

#[post("/log_in")]
pub async fn log_in_submit(
    req: HttpRequest,
    form: web::Form<LoginForm>,
    ayb_config: web::Data<AybConfig>,
) -> Result<HttpResponse> {
    let client = init_ayb_client(&ayb_config, &req);

    match client.log_in(&form.username).await {
        Ok(_) => Ok(HttpResponse::Ok()
            .content_type("text/html; charset=utf-8")
            .body(render("log_in_check_email.html", &tera::Context::new()))),
        Err(_) => Ok(HttpResponse::Ok()
            .content_type("text/html; charset=utf-8")
            .body(render("log_in_error.html", &tera::Context::new()))),
    }
}
