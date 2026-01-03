use crate::server::config::AybConfig;
use crate::server::ui_endpoints::auth::{authentication_details, init_ayb_client};
use crate::server::ui_endpoints::templates::{error_snippet, ok_response};
use actix_web::{delete, get, web, HttpRequest, HttpResponse, Result};

// Note: We use /-/tokens to avoid conflicts with databases named "tokens"
#[get("/{entity}/-/tokens")]
pub async fn entity_tokens(
    req: HttpRequest,
    path: web::Path<String>,
    ayb_config: web::Data<AybConfig>,
) -> Result<HttpResponse> {
    let entity_slug = path.into_inner().to_lowercase();
    let client = init_ayb_client(&ayb_config, &req);

    match client.list_tokens().await {
        Ok(token_list) => {
            let mut context = tera::Context::new();
            context.insert("entity", &entity_slug);
            context.insert("tokens", &token_list.tokens);
            context.insert(
                "logged_in_entity",
                &authentication_details(&req).map(|details| details.entity),
            );
            ok_response("entity_tokens.html", &context)
        }
        Err(err) => error_snippet("Error loading tokens", &err.to_string()),
    }
}

#[delete("/{entity}/-/tokens/{short_token}")]
pub async fn revoke_token(
    req: HttpRequest,
    path: web::Path<(String, String)>,
    ayb_config: web::Data<AybConfig>,
) -> Result<HttpResponse> {
    let (_entity_slug, short_token) = path.into_inner();
    let client = init_ayb_client(&ayb_config, &req);

    match client.revoke_token(&short_token).await {
        Ok(_) => Ok(HttpResponse::Ok()
            .content_type("text/html")
            .body(format!(r#"<tr class="text-muted-foreground"><td colspan="7" class="text-center italic py-2">Token {} revoked successfully</td></tr>"#, short_token))),
        Err(err) => error_snippet("Error revoking token", &err.to_string()),
    }
}
