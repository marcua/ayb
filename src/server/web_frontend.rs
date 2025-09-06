use crate::error::AybError;
use crate::server::config::{AybConfig, WebHostingMethod};
use crate::templating::TemplateString;
use serde::Deserialize;
use url::Url;

#[derive(Clone)]
pub struct WebFrontendDetails {
    base_url: Url,
    public_base_url: Url,
    endpoints: WebFrontendEndpoints,
}

#[derive(Clone, Deserialize)]
pub struct WebFrontendEndpoints {
    profile: TemplateString,
    confirmation: TemplateString,
}

#[derive(Deserialize)]
struct RemoteWebDetails {
    base_url: Url,
    endpoints: WebFrontendEndpoints,
}

pub fn local_base_url(config: &AybConfig) -> String {
    format!("http://localhost:{}", config.port)
}

pub fn public_base_url(config: &AybConfig) -> String {
    if let Some(ref public_url) = config.public_url {
        public_url.clone()
    } else {
        local_base_url(config)
    }
}

impl WebFrontendDetails {
    async fn from_url(url: &Url) -> Result<Self, AybError> {
        let remote: RemoteWebDetails = reqwest::get(url.to_string()).await?.json().await?;
        Ok(WebFrontendDetails {
            base_url: remote.base_url.clone(),
            public_base_url: remote.base_url,
            endpoints: remote.endpoints,
        })
    }

    fn from_local(config: &AybConfig) -> Self {
        WebFrontendDetails {
            base_url: Url::parse(&local_base_url(config)).unwrap(),
            public_base_url: Url::parse(&public_base_url(config)).unwrap(),
            endpoints: WebFrontendEndpoints {
                profile: TemplateString {
                    string: "{entity}".into(),
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
            .public_base_url
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
            .public_base_url
            .join(&relative)
            .expect("invalid confirmation template string provided by the web frontend");
        absolute.to_string()
    }

    pub async fn load(config: AybConfig) -> Result<Option<Self>, AybError> {
        if let Some(ref web_conf) = config.web {
            Ok(Some(match web_conf.hosting_method {
                WebHostingMethod::Remote => {
                    if let Some(base_url) = &web_conf.base_url {
                        Self::from_url(base_url).await?
                    } else {
                        return Err(AybError::ConfigurationError {
                            message: "Remote web hosting method requires a base_url".to_string(),
                        });
                    }
                }
                WebHostingMethod::Local => Self::from_local(&config),
            }))
        } else {
            Ok(None)
        }
    }
}
