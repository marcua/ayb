use crate::http::structs::AybConfigEmail;
use crate::templating::TemplateString;

pub fn render_confirmation_template(config: &AybConfigEmail, token: &str) -> String {
    let cli_confirm_tmpl: TemplateString =
        "To complete your registration, type\n\tayb client confirm {token}"
            .to_string()
            .into();
    let web_confirm_tmpl: TemplateString = "To complete your registration, visit\n\t {url}"
        .to_string()
        .into();

    if let Some(tmpl_conf) = &config.templates {
        if let Some(confirm_conf) = &tmpl_conf.confirm {
            return web_confirm_tmpl.execute(vec![(
                "url",
                &confirm_conf
                    .confirmation_url
                    .execute(vec![("token", &urlencoding::encode(token))]),
            )]);
        }
    }

    cli_confirm_tmpl.execute(vec![("token", token)])
}
