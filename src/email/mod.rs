use crate::email::templating::render_confirmation_template;
use crate::error::AybError;
use crate::http::structs::AybConfigEmail;
use crate::http::web_frontend::WebFrontendDetails;
use lettre::{
    message::header::ContentType,
    transport::smtp::authentication::Credentials,
    transport::smtp::client::{Tls, TlsParameters},
    AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor,
};

mod templating;

pub async fn send_registration_email(
    to: &str,
    token: &str,
    config: &AybConfigEmail,
    web_details: &Option<WebFrontendDetails>,
    e2e_testing_on: bool,
) -> Result<(), AybError> {
    send_email(
        to,
        "Your login credentials",
        render_confirmation_template(web_details, token),
        config,
        e2e_testing_on,
    )
    .await
}

async fn send_email(
    to: &str,
    subject: &str,
    body: String,
    config: &AybConfigEmail,
    e2e_testing_on: bool,
) -> Result<(), AybError> {
    let email = Message::builder()
        .from(config.from.parse()?)
        .reply_to(config.reply_to.parse()?)
        .to(to.parse()?)
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
