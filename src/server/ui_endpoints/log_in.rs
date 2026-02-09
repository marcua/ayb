use crate::server::config::AybConfig;
use crate::server::ui_endpoints::auth::init_ayb_client;
use crate::server::ui_endpoints::templates::ok_response;
use actix_web::{get, post, web, HttpRequest, HttpResponse, Result};

#[derive(serde::Deserialize)]
pub struct LoginQuery {
    next: Option<String>,
}

#[get("/log_in")]
pub async fn log_in(query: web::Query<LoginQuery>) -> Result<HttpResponse> {
    let mut context = tera::Context::new();
    if let Some(ref next) = query.next {
        context.insert("next", next);
    }
    ok_response("log_in.html", &context)
}

#[derive(serde::Deserialize)]
pub struct LoginForm {
    username: String,
    next: Option<String>,
}

#[post("/log_in")]
pub async fn log_in_submit(
    req: HttpRequest,
    form: web::Form<LoginForm>,
    ayb_config: web::Data<AybConfig>,
) -> Result<HttpResponse> {
    let client = init_ayb_client(&ayb_config, &req);

    match client.log_in(&form.username).await {
        Ok(_) => {
            let mut context = tera::Context::new();
            if let Some(ref next) = form.next {
                context.insert("next", next);
            }
            ok_response("log_in_check_email.html", &context)
        }
        Err(_) => ok_response("log_in_error.html", &tera::Context::new()),
    }
}
