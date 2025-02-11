use super::templates::{base_template, create_client};
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
            let content = r#"
                <div class="bg-white rounded-lg shadow-sm p-6">
                    <div class="bg-green-50 border border-green-200 text-green-700 px-4 py-3 rounded mb-4">
                        Your email has been confirmed! You are now logged in.
                    </div>
// AI! The path on the next line isn't being filled in with the entity's username.
                    <a href="/d/{}" 
                       class="block w-full py-2 px-4 border border-transparent rounded-md shadow-sm text-center text-white bg-blue-600 hover:bg-blue-700">
                        Go to Your Profile
                    </a>
                </div>
            "#;
            api_token.entity;

            Ok(HttpResponse::Ok()
                .content_type("text/html; charset=utf-8")
                .append_header((
                    "Set-Cookie",
                    format!("auth={}; Path=/; HttpOnly", api_token.token),
                ))
                .body(base_template("Email Confirmed", content)))
        }
        Err(_) => {
            let content = r#"
                <div class="bg-white rounded-lg shadow-sm p-6">
                    <div class="bg-red-50 border border-red-200 text-red-700 px-4 py-3 rounded">
                        Invalid or expired confirmation link. Please try logging in again to receive a new link.
                    </div>
                </div>
            "#;

            Ok(HttpResponse::Ok()
                .content_type("text/html; charset=utf-8")
                .body(base_template("Confirmation Failed", content)))
        }
    }
}
