use crate::error::AybError;
use crate::http::structs::AybConfigEmail;
use lettre::{
    message::header::ContentType, transport::smtp::authentication::Credentials, AsyncSmtpTransport,
    AsyncTransport, Message, Tokio1Executor,
};

pub async fn send_registration_email(
    to: &str,
    token: &str,
    config: &AybConfigEmail,
) -> Result<(), AybError> {
    return send_email(
        to,
        "Your login credentials",
        format!("To log in, type stacks client email-confirm email@example.com {token}"),
        config,
    )
    .await;
}

async fn send_email(
    to: &str,
    subject: &str,
    body: String,
    config: &AybConfigEmail,
) -> Result<(), AybError> {
    // TODO(marcua): Any way to be more careful about these unwraps?
    let email = Message::builder()
        .from(config.from.parse().unwrap())
        .reply_to(config.reply_to.parse().unwrap())
        .to(to.parse().unwrap())
        .subject(subject)
        .header(ContentType::TEXT_PLAIN)
        .body(body)
        .unwrap();

    let creds = Credentials::new(
        config.smtp_username.to_owned(),
        config.smtp_password.to_owned(),
    );

    // Open a remote connection to gmail
    let mailer: AsyncSmtpTransport<Tokio1Executor> =
        AsyncSmtpTransport::<Tokio1Executor>::relay(&config.smtp_host)
            .unwrap()
            .credentials(creds)
            .build();

    if let Err(e) = mailer.send(email).await {
        return Err(AybError {
            message: format!("Could not send email: {e:?}"),
        });
    }

    Ok(())
}
