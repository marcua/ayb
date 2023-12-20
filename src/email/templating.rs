use crate::templating::TemplateString;
use crate::http::web_frontend::WebFrontendDetails;

pub fn render_confirmation_template(web_details: &Option<WebFrontendDetails>, token: &str) -> String {
    let cli_confirm_tmpl: TemplateString =
        "To complete your registration, type\n\tayb client confirm {token}"
            .to_string()
            .into();
    let web_confirm_tmpl: TemplateString = "To complete your registration, visit\n\t {url}"
        .to_string()
        .into();

    if let Some(web_details) = web_details {
        return web_confirm_tmpl.execute(vec![("url", &web_details.confirmation(token))]);
    }

    cli_confirm_tmpl.execute(vec![("token", token)])
}
