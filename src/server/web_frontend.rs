use crate::error::AybError;
use crate::server::config::{AybConfigWeb, WebHostingMethod};
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
    async fn from_url(url: &Url) -> Result<Self, AybError> {
        Ok(reqwest::get(url.to_string()).await?.json().await?)
    }

    fn from_local(base_url: Url) -> Self {
        WebFrontendDetails {
            base_url,
            endpoints: WebFrontendEndpoints {
                profile: TemplateString {
                    string: "d/{entity}".into(),
                },
                confirmation: TemplateString {
                    string: "confirm/{token}".into(),
                },
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

    pub async fn load(web_conf: AybConfigWeb) -> Result<Self, AybError> {
        match web_conf.hosting_method {
            WebHostingMethod::Remote => Self::from_url(&web_conf.base_url).await,
            WebHostingMethod::Local => Ok(Self::from_local(web_conf.base_url)),
        }
    }
}
