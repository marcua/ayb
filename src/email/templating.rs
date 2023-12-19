use crate::templating::TemplateString;
use crate::web_info::WebInfo;

pub fn render_confirmation_template(web_info: &Option<WebInfo>, token: &str) -> String {
    let cli_confirm_tmpl: TemplateString =
        "To complete your registration, type\n\tayb client confirm {token}"
            .to_string()
            .into();
    let web_confirm_tmpl: TemplateString = "To complete your registration, visit\n\t {url}"
        .to_string()
        .into();

    if let Some(web_info) = web_info {
        return web_confirm_tmpl.execute(vec![("url", &web_info.confirmation(token))]);
    }

    cli_confirm_tmpl.execute(vec![("token", token)])
}
