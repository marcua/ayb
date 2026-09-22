use crate::server::config::AybConfig;
use crate::server::ui_endpoints::auth::{cookie_for_token, init_ayb_client};
use crate::server::ui_endpoints::templates::{ok_response, render};
use actix_web::{get, post, web, HttpRequest, HttpResponse, Result};

/// Confirmation interstitial. This is deliberately non-consuming: email
/// scanners and link prefetchers follow the link, and a GET must not burn
/// the token. The user either confirms here (POST) or copies the code into
/// the browser/window where they started logging in.
#[get("/confirm/{token}")]
pub async fn confirm(path: web::Path<String>) -> Result<HttpResponse> {
    let token = path.into_inner();
    let mut context = tera::Context::new();
    context.insert("token", &token);
    ok_response("confirm.html", &context)
}

#[derive(serde::Deserialize)]
pub struct ConfirmForm {
    token: String,
}

/// Consume a confirmation code and set the auth cookie. Used by both the
/// interstitial's "Confirm in this browser" button and the paste field on
/// the check-email pages.
#[post("/confirm")]
pub async fn confirm_submit(
    req: HttpRequest,
    form: web::Form<ConfirmForm>,
    ayb_config: web::Data<AybConfig>,
) -> Result<HttpResponse> {
    let client = init_ayb_client(&ayb_config, &req);

    match client.confirm(&form.token).await {
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
