use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tokio::io::AsyncWriteExt;

use crate::error::AybError;
use crate::server::config::{AybConfigEmailFile, AybConfigEmailSmtp};
use lettre::{
    message::header::ContentType, transport::smtp::authentication::Credentials, AsyncSmtpTransport,
    AsyncTransport, Message, Tokio1Executor,
};

#[derive(Serialize, Deserialize, Debug)]
pub struct EmailEntry {
    pub from: String,
    pub to: String,
    pub reply_to: String,
    pub subject: String,
    pub content_type: String,
    pub content_transfer_encoding: String,
    pub date: String,
    pub content: Vec<String>,
}

#[async_trait]
pub trait EmailBackend {
    async fn send_email(
        &self,
        to: &str,
        subject: &str,
        body: &str,
        from: &str,
        reply_to: &str,
    ) -> Result<(), AybError>;
}

pub struct SmtpBackend {
    config: AybConfigEmailSmtp,
}

impl SmtpBackend {
    pub fn new(config: AybConfigEmailSmtp) -> Self {
        Self { config }
    }
}

#[async_trait]
impl EmailBackend for SmtpBackend {
    async fn send_email(
        &self,
        to: &str,
        subject: &str,
        body: &str,
        from: &str,
        reply_to: &str,
    ) -> Result<(), AybError> {
        let email = Message::builder()
            .from(from.parse()?)
            .reply_to(reply_to.parse()?)
            .to(to.parse()?)
            .subject(subject)
            .header(ContentType::TEXT_PLAIN)
            .body(body.to_string())
            .unwrap();

        let creds = Credentials::new(
            self.config.smtp_username.to_owned(),
            self.config.smtp_password.to_owned(),
        );

        let mailer: AsyncSmtpTransport<Tokio1Executor> =
            AsyncSmtpTransport::<Tokio1Executor>::relay(&self.config.smtp_host)
                .unwrap()
                .credentials(creds)
                .port(self.config.smtp_port)
                .build();

        if let Err(e) = mailer.send(email).await {
            return Err(AybError::Other {
                message: format!("Could not send email: {e:?}"),
            });
        }

        Ok(())
    }
}

pub struct FileBackend {
    config: AybConfigEmailFile,
}

impl FileBackend {
    pub fn new(config: AybConfigEmailFile) -> Self {
        Self { config }
    }
}

#[async_trait]
impl EmailBackend for FileBackend {
    async fn send_email(
        &self,
        to: &str,
        subject: &str,
        body: &str,
        from: &str,
        reply_to: &str,
    ) -> Result<(), AybError> {
        let email_entry = EmailEntry {
            from: from.to_string(),
            to: to.to_string(),
            reply_to: reply_to.to_string(),
            subject: subject.to_string(),
            content_type: "text/plain".to_string(),
            content_transfer_encoding: "7bit".to_string(),
            date: chrono::Utc::now().to_rfc2822(),
            content: body.lines().map(|s| s.to_string()).collect(),
        };

        let json_line = serde_json::to_string(&email_entry).map_err(|e| AybError::Other {
            message: format!("Failed to serialize email: {e:?}"),
        })? + "\n";

        // Create parent directory if it doesn't exist
        if let Some(parent) = std::path::Path::new(&self.config.path).parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|e| AybError::Other {
                    message: format!("Failed to create email directory: {e:?}"),
                })?;
        }

        // Append JSON line to file
        let mut file = tokio::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.config.path)
            .await
            .map_err(|e| AybError::Other {
                message: format!("Failed to open email file: {e:?}"),
            })?;

        file.write_all(json_line.as_bytes())
            .await
            .map_err(|e| AybError::Other {
                message: format!("Failed to write email to file: {e:?}"),
            })?;

        Ok(())
    }
}

pub struct MultiBackend {
    backends: Vec<Box<dyn EmailBackend + Send + Sync>>,
}

impl MultiBackend {
    pub fn new(backends: Vec<Box<dyn EmailBackend + Send + Sync>>) -> Self {
        Self { backends }
    }
}

#[async_trait]
impl EmailBackend for MultiBackend {
    async fn send_email(
        &self,
        to: &str,
        subject: &str,
        body: &str,
        from: &str,
        reply_to: &str,
    ) -> Result<(), AybError> {
        let mut errors = Vec::new();

        for backend in &self.backends {
            if let Err(e) = backend.send_email(to, subject, body, from, reply_to).await {
                errors.push(e);
            }
        }

        // Return error only if ALL backends fail
        if errors.len() == self.backends.len() && !errors.is_empty() {
            return Err(AybError::Other {
                message: format!("All email backends failed: {:?}", errors),
            });
        }

        Ok(())
    }
}
