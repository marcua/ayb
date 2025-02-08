use serde::Serialize;

#[derive(Serialize)]
pub struct WebEndpoints {
    profile: String,
    confirmation: String,
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
