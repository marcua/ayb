use crate::error::AybError;
use crate::templating::TemplateString;
use serde::Deserialize;
use url::Url;

#[derive(Clone, Deserialize)]
pub struct WebInfo {
    base_url: Url,
    endpoints: WebEndpoints,
}

#[derive(Clone, Deserialize)]
pub struct WebEndpoints {
    profile: TemplateString,
    confirmation: TemplateString,
}

impl WebInfo {
    pub async fn from_url(url: &Url) -> Result<Self, AybError> {
        Ok(reqwest::get(url.to_string()).await?.json().await?)
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
