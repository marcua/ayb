use crate::http::structs::AybConfigEmail;

const CLI_CONFIRM_TMPL: &str = "To complete your registration, type\n\tayb client confirm {token}";
const WEB_CONFIRM_TMPL: &str = "To complete your registration, visit\n\t {url}";

pub fn render_confirmation_template(config: &AybConfigEmail, token: &str) -> String {
    if let Some(tmpl_conf) = &config.templates {
        if let Some(confirm_conf) = &tmpl_conf.confirm {
            return WEB_CONFIRM_TMPL.replace(
                "{url}",
                &confirm_conf
                    .confirmation_url
                    .replace("{token}", &urlencoding::encode(token)),
            );
        }
    }

    CLI_CONFIRM_TMPL.replace("{token}", token)
}
