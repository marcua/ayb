use crate::server::config::{public_base_url, AybConfig};
use crate::templating::TemplateString;

pub fn render_confirmation_template(ayb_config: &AybConfig, token: &str) -> String {
    let tmpl: TemplateString = "To complete your registration, visit\n\t{url}\n\n\
                                Or type\n\tayb client confirm {token}"
        .to_string()
        .into();
    let confirmation_url = format!(
        "{}/confirm/{}",
        public_base_url(ayb_config),
        urlencoding::encode(token),
    );
    tmpl.execute(vec![("url", &confirmation_url), ("token", token)])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::config::{
        AybConfig, AybConfigAuthentication, AybConfigCors, AybConfigEmailBackends,
    };

    fn config_with_public_url(public_url: Option<String>, port: u16) -> AybConfig {
        AybConfig {
            host: "localhost".to_string(),
            port,
            public_url,
            database_url: "sqlite://test.db".to_string(),
            data_path: "./test_data".to_string(),
            authentication: AybConfigAuthentication {
                fernet_key: "test_key".to_string(),
                token_expiration_seconds: 3600,
            },
            email: AybConfigEmailBackends {
                smtp: None,
                file: Some(crate::server::config::AybConfigEmailFile {
                    path: "./test_emails.jsonl".to_string(),
                }),
            },
            cors: AybConfigCors {
                origin: "*".to_string(),
            },
            snapshots: None,
        }
    }

    #[test]
    fn test_render_confirmation_local() {
        let config = config_with_public_url(None, 5433);
        let token = "test_token_456";
        let result = render_confirmation_template(&config, token);

        assert_eq!(
            result,
            "To complete your registration, visit\n\thttp://localhost:5433/confirm/test_token_456\n\n\
             Or type\n\tayb client confirm test_token_456"
        );
    }

    #[test]
    fn test_render_confirmation_with_public_url() {
        let config = config_with_public_url(Some("https://ayb.example.com".to_string()), 8080);
        let token = "test_token_789";
        let result = render_confirmation_template(&config, token);

        assert_eq!(
            result,
            "To complete your registration, visit\n\thttps://ayb.example.com/confirm/test_token_789\n\n\
             Or type\n\tayb client confirm test_token_789"
        );
    }
}
