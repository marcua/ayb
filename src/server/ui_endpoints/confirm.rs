use crate::server::config::AybConfig;
use crate::server::ui_endpoints::auth::{cookie_for_token, init_ayb_client};
use crate::server::ui_endpoints::templates::{ok_response, render};
use actix_web::{get, post, web, HttpRequest, HttpResponse, Result};

/// Shows a page instead of confirming. Email scanners and link
/// prefetchers open links automatically, and if a GET confirmed the code,
/// a scanner would finish registration (proving ownership of the email)
/// and create an API token nobody asked for. The user confirms with a POST
/// from this page, or pastes the code into the window where they started.
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
/// interstitial's "Continue in this browser" button and the paste field on
/// the check-email pages.
#[post("/confirm")]
pub async fn confirm_submit(
    req: HttpRequest,
    form: web::Form<ConfirmForm>,
    ayb_config: web::Data<AybConfig>,
) -> Result<HttpResponse> {
    // Reject cross-site submissions so another site can't log a visitor
    // into the attacker's account by auto-submitting the attacker's code.
    // Browsers set Sec-Fetch-Site on every request; non-browser clients
    // omit it and are unaffected.
    let fetch_site = req
        .headers()
        .get("Sec-Fetch-Site")
        .and_then(|v| v.to_str().ok());
    if matches!(fetch_site, Some(site) if site != "same-origin" && site != "none") {
        return Ok(HttpResponse::Forbidden()
            .content_type("text/html; charset=utf-8")
            .body(render("confirm_error.html", &tera::Context::new())));
    }

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
