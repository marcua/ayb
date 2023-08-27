use crate::error::AybError;
use crate::http::structs::AybConfigEmail;
use lettre::{
    message::header::ContentType, transport::smtp::authentication::Credentials, transport::smtp::client::{Tls, TlsParameters}, AsyncSmtpTransport,
    AsyncTransport, Message, SmtpTransport, Transport, Tokio1Executor,
};

pub async fn send_registration_email(
    to: &str,
    token: &str,
    config: &AybConfigEmail,
) -> Result<(), AybError> {
    return send_email(
        to,
        "Your login credentials",
        format!("To log in, type\n\tayb client confirm {token}"),
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

    // Open a remote connection to SMTP server
    if config.smtp_host == "localhost" {
        // TODO(marcua): See if you can use the Async transport in Rust for both use cases
        // TODO(marcua): Introduce an e2e config option for server, get rid of hard-coded hostname/port
        // TODO(marcua): Make Python write to file
        // TODO(marcua): Make e2e tests read file, assert emails work
        let tls = TlsParameters::builder(config.smtp_host.to_owned())
            .dangerous_accept_invalid_certs(true)
            .build()
            .unwrap();
        let mailer = SmtpTransport::relay(&config.smtp_host)
            .unwrap()
            .port(10025)
            .tls(Tls::Required(tls))
            //.credentials(creds)
            .build();
        /*let mailer: AsyncSmtpTransport<Tokio1Executor> =        
            AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(&config.smtp_host)
            //.unwrap()
            .credentials(creds.clone())
            .port(config.smtp_port)
            .tls(Tls::Required(tls))
            .build();*/
        
        if let Err(e) = mailer.send(&email) {
            return Err(AybError {
                message: format!("Could not send email: {e:?}"),
            });
        }
        
    } else {
        let mailer: AsyncSmtpTransport<Tokio1Executor> =
            AsyncSmtpTransport::<Tokio1Executor>::relay(&config.smtp_host)
            .unwrap()
            .credentials(creds.clone())
            .build();
        if let Err(e) = mailer.send(email).await {
            return Err(AybError {
                message: format!("Could not send email: {e:?}"),
            });
        }
    }

    Ok(())
}
