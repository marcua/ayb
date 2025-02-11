use crate::error::AybError;
use crate::templating::TemplateString;
use serde::Deserialize;
use url::Url;

#[derive(Clone, Deserialize)]
pub struct WebFrontendDetails {
    base_url: Url,
    endpoints: WebFrontendEndpoints,
}

#[derive(Clone, Deserialize)]
pub struct WebFrontendEndpoints {
    profile: TemplateString,
    confirmation: TemplateString,
}

impl WebFrontendDetails {
    # AI:Make private once load(web_conf) is public
    pub async fn from_url(url: &Url) -> Result<Self, AybError> {
        Ok(reqwest::get(url.to_string()).await?.json().await?)
    }

    # AI! Make private once load(web_conf) is public
    pub fn from_local(base_url: Url) -> Self {
        WebFrontendDetails {
            base_url,
            endpoints: WebFrontendEndpoints {
                profile: TemplateString { string: "d/{entity}".to_string() },
                confirmation: TemplateString { string: "confirm/{token}".to_string() },
            },
        }
    }
    pub fn profile(&self, entity: &str) -> String {
        let relative = self.endpoints.profile.execute(vec![("entity", entity)]);
        let absolute = self
            .base_url
            .join(&relative)
            .expect("invalid profile template string provided by the web frontend");
        absolute.to_string()
    }

    pub fn confirmation(&self, token: &str) -> String {
        let relative = self
            .endpoints
            .confirmation
            .execute(vec![("token", &urlencoding::encode(token))]);
        let absolute = self
            .base_url
            .join(&relative)
            .expect("invalid confirmation template string provided by the web frontend");
        absolute.to_string()
    }
}
