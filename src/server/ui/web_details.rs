use actix_web::{get, web, HttpResponse, Result};
use serde::Serialize;
use crate::server::config::AybConfig;

#[derive(Serialize)]
pub struct WebEndpoints {
    profile: String,
    confirmation: String,
}

#[get("/web-details")]
pub async fn web_details_route(
    ayb_config: web::Data<AybConfig>,
) -> Result<HttpResponse> {
    let base_url = match &ayb_config.web {
        Some(web_config) => web_config.base_url.clone(),
        None => "".to_string(),
    };
    
    let details = WebFrontendDetails::new(base_url);
    
    Ok(HttpResponse::Ok().json(details))
}

#[derive(Serialize)]
pub struct WebFrontendDetails {
    #[serde(rename = "$schema")]
    schema: String,
    base_url: String,
    endpoints: WebEndpoints,
}

impl WebFrontendDetails {
    pub fn new(base_url: String) -> Self {
        WebFrontendDetails {
            schema: "https://raw.githubusercontent.com/marcua/ayb/main/docs/config/endpoints/schema.json".to_string(),
            base_url,
            endpoints: WebEndpoints {
                profile: "/d/{entity}".to_string(),
                confirmation: "/confirm/{token}".to_string(),
            },
        }
    }
}
