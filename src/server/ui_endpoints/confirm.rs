use crate::server::config::AybConfig;
use crate::server::ui_endpoints::auth::{cookie_for_token, init_ayb_client};
use crate::server::ui_endpoints::templates::{ok_response, render};
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
            let mut context = tera::Context::new();
            context.insert("entity", &api_token.entity);
            Ok(HttpResponse::Ok()
                .content_type("text/html; charset=utf-8")
                .append_header(("Set-Cookie", cookie_for_token(&api_token)))
                .body(render("confirm_success.html", &context)))
        }
        Err(_) => ok_response("confirm_error.html", &tera::Context::new()),
    }
}
