use crate::email::backend::{EmailBackend, FileBackend, MultiBackend, SmtpBackend};
use crate::email::templating::render_confirmation_template;
use crate::error::AybError;
use crate::server::config::AybConfigEmailBackends;
use crate::server::web_frontend::WebFrontendDetails;

mod backend;
mod templating;

pub async fn send_registration_email(
    email_backends: &AybConfigEmailBackends,
    to: &str,
    token: &str,
    web_details: &Option<WebFrontendDetails>,
) -> Result<(), AybError> {
    let backend = create_email_backend(email_backends)?;

    // Get from/reply_to from SMTP config if available, or use defaults
    let (from, reply_to) = get_email_addresses(email_backends);

    let body = render_confirmation_template(web_details, token);

    backend
        .send_email(to, "Your login credentials", &body, &from, &reply_to)
        .await
}

fn create_email_backend(
    config: &AybConfigEmailBackends,
) -> Result<Box<dyn EmailBackend + Send + Sync>, AybError> {
    config.validate()?;

    let mut backends: Vec<Box<dyn EmailBackend + Send + Sync>> = Vec::new();

    if let Some(smtp_config) = &config.smtp {
        backends.push(Box::new(SmtpBackend::new(smtp_config.clone())));
    }

    if let Some(file_config) = &config.file {
        backends.push(Box::new(FileBackend::new(file_config.clone())));
    }

    Ok(Box::new(MultiBackend::new(backends)))
}

fn get_email_addresses(config: &AybConfigEmailBackends) -> (String, String) {
    if let Some(smtp) = &config.smtp {
        (smtp.from.clone(), smtp.reply_to.clone())
    } else {
        // Fallback defaults when only file backend is configured
        (
            "ayb <noreply@localhost>".to_string(),
            "ayb <noreply@localhost>".to_string(),
        )
    }
}
