use crate::server::ui_endpoints::auth::COOKIE_FOR_LOGOUT;
use actix_web::{get, HttpResponse, Result};

#[get("/log_out")]
pub async fn log_out() -> Result<HttpResponse> {
    Ok(HttpResponse::Found()
        .append_header(("Location", "/log_in"))
        .append_header(("Set-Cookie", COOKIE_FOR_LOGOUT))
        .finish())
}
