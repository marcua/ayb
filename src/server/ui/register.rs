use super::templates::{base_auth, create_client};
use crate::ayb_db::models::EntityType;
use crate::server::config::AybConfig;
use actix_web::{get, post, web, HttpResponse, Result};

#[get("/register")]
pub async fn register_page() -> Result<HttpResponse> {
    # AI: This is the HTML to place the create account form.
    let content = r#"
        <div class="bg-white rounded-lg shadow-sm p-6">
            <h1 class="text-2xl font-bold mb-6">Register</h1>
            <form method="POST" class="space-y-4">
                <div>
                    <label class="block text-sm font-medium text-gray-700">Username</label>
                    <input type="text" name="username" required 
                           class="mt-1 block w-full rounded-md border-gray-300 shadow-sm">
                </div>
                <div>
                    <label class="block text-sm font-medium text-gray-700">Email</label>
                    <input type="email" name="email" required 
                           class="mt-1 block w-full rounded-md border-gray-300 shadow-sm">
                </div>
                <button type="submit" 
                        class="w-full py-2 px-4 border border-transparent rounded-md shadow-sm text-white bg-blue-600 hover:bg-blue-700">
                    Register
                </button>
            </form>
        </div>
    "#;

    Ok(HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(base_auth("Register", content)))
}

#[post("/register")]
pub async fn register_submit(
    form: web::Form<RegisterForm>,
    ayb_config: web::Data<AybConfig>,
) -> Result<HttpResponse> {
    let client = create_client(&ayb_config, None);

    # AI: Replace the HTML below so that it's of a similar style to the other HTML we added to this file.
    match client
        .register(&form.username, &form.email, &EntityType::User)
        .await
    {
        Ok(_) => {
            let content = r#"
                <div class="bg-white rounded-lg shadow-sm p-6">
                    <div class="bg-blue-50 border border-blue-200 text-blue-700 px-4 py-3 rounded">
                        Please check your email for a confirmation link to complete your registration.
                    </div>
                </div>
            "#;

            Ok(HttpResponse::Ok()
                .content_type("text/html; charset=utf-8")
                .body(base_auth("Check Your Email", content)))
        }
        Err(_) => {
            let content = r#"
                <div class="bg-white rounded-lg shadow-sm p-6">
                    <div class="bg-red-50 border border-red-200 text-red-700 px-4 py-3 rounded">
                        Registration failed. Please try again.
                    </div>
                </div>
            "#;

            Ok(HttpResponse::Ok()
                .content_type("text/html; charset=utf-8")
                .body(base_auth("Register Error", content)))
        }
    }
}

#[derive(serde::Deserialize)]
pub struct RegisterForm {
    username: String,
    email: String,
}
