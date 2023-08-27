use crate::error::AybError;
use crate::http::structs::AybConfigEmail;
use lettre::{
    message::header::ContentType,
    transport::smtp::authentication::Credentials,
    transport::smtp::client::{Tls, TlsParameters},
    AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor,
};

pub async fn send_registration_email(
    to: &str,
    token: &str,
    config: &AybConfigEmail,
    e2e_testing_on: bool,
) -> Result<(), AybError> {
    return send_email(
        to,
        "Your login credentials",
        format!("To log in, type\n\tayb client confirm {token}"),
        config,
        e2e_testing_on,
    )
    .await;
}

async fn send_email(
    to: &str,
    subject: &str,
    body: String,
    config: &AybConfigEmail,
    e2e_testing_on: bool,
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

    let mut mailer: AsyncSmtpTransport<Tokio1Executor> =
        AsyncSmtpTransport::<Tokio1Executor>::relay(&config.smtp_host)
            .unwrap()
            .credentials(creds.clone())
            .port(config.smtp_port)
            .build();

    if e2e_testing_on {
        // When end-to-end testing, we connect to a local SMTP server
        // that does not verify credentials or sign certificates with
        // a certificate authority.

        // TODO(marcua): Make e2e tests read file, assert emails work
        let tls = TlsParameters::builder(config.smtp_host.to_owned())
            .dangerous_accept_invalid_certs(true)
            .build()
            .unwrap();
        mailer = AsyncSmtpTransport::<Tokio1Executor>::relay(&config.smtp_host)
            .unwrap()
            .port(config.smtp_port)
            .tls(Tls::Required(tls))
            .build();
    }

    if let Err(e) = mailer.send(email).await {
        return Err(AybError {
            message: format!("Could not send email: {e:?}"),
        });
    }

    Ok(())
}
