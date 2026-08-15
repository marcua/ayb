use crate::email::backend::{EmailBackends, FileBackend, SmtpBackend};
use crate::email::templating::render_confirmation_template;
use crate::error::AybError;
use crate::server::config::{AybConfig, AybConfigEmailBackends};

pub mod backend;
mod templating;

pub async fn send_registration_email(
    email_backends: &EmailBackends,
    ayb_config: &AybConfig,
    to: &str,
    token: &str,
) -> Result<(), AybError> {
    // Get from/reply_to from SMTP config if available, or use defaults
    let (from, reply_to) = get_email_addresses(&ayb_config.email);

    let body = render_confirmation_template(ayb_config, token);

    email_backends
        .send_email(to, "Your login credentials", &body, &from, &reply_to)
        .await
}

pub fn create_email_backends(config: &AybConfigEmailBackends) -> EmailBackends {
    let smtp_backend = config
        .smtp
        .as_ref()
        .map(|smtp_config| SmtpBackend::new(smtp_config.clone()));
    let file_backend = config
        .file
        .as_ref()
        .map(|file_config| FileBackend::new(file_config.clone()));

    EmailBackends::new(smtp_backend, file_backend)
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
