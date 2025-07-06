use crate::server::web_frontend::WebFrontendDetails;
use crate::templating::TemplateString;

pub fn render_confirmation_template(
    web_details: &Option<WebFrontendDetails>,
    token: &str,
) -> String {
    if let Some(web_details) = web_details {
        let both_confirm_tmpl: TemplateString = "To complete your registration, visit\n\t{url}\n\n\
                                                 Or type\n\tayb client confirm {token}"
            .to_string()
            .into();
        return both_confirm_tmpl.execute(vec![
            ("url", &web_details.confirmation(token)),
            ("token", token),
        ]);
    }

    let cli_confirm_tmpl: TemplateString =
        "To complete your registration, type\n\tayb client confirm {token}"
            .to_string()
            .into();
    cli_confirm_tmpl.execute(vec![("token", token)])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::config::{
        AybConfig, AybConfigAuthentication, AybConfigCors, AybConfigEmailBackends, AybConfigWeb,
        WebHostingMethod,
    };
    use crate::server::web_frontend::WebFrontendDetails;

    #[tokio::test]
    async fn test_render_confirmation_without_web() {
        let token = "test_token_123";
        let result = render_confirmation_template(&None, token);

        assert_eq!(
            result,
            "To complete your registration, type\n\tayb client confirm test_token_123"
        );
    }

    #[tokio::test]
    async fn test_render_confirmation_with_web() {
        // Create a minimal config for WebFrontendDetails::from_local
        let config = AybConfig {
            host: "localhost".to_string(),
            port: 5433,
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
            web: Some(AybConfigWeb {
                hosting_method: WebHostingMethod::Local,
                base_url: None,
            }),
            cors: AybConfigCors {
                origin: "*".to_string(),
            },
            snapshots: None,
            isolation: None,
        };

        let web_details = WebFrontendDetails::load(config).await.unwrap();
        let token = "test_token_456";
        let result = render_confirmation_template(&web_details, token);

        assert_eq!(
            result,
            "To complete your registration, visit\n\thttp://localhost:5433/confirm/test_token_456\n\n\
             Or type\n\tayb client confirm test_token_456"
        );
    }
}
