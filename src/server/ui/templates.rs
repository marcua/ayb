fn base_template(title: &str, content: &str) -> String {
    format!(
        r#"<!DOCTYPE html>
<html
  lang="en"
  class="uk-theme-blue uk-radii-sm uk-shadows-md uk-font-sm">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>{} - ayb</title>
    <link rel="stylesheet" href="https://unpkg.com/franken-ui@2.0.0-internal.42/dist/css/core.min.css"/>
    <link rel="stylesheet" href="https://unpkg.com/franken-ui@2.0.0-internal.42/dist/css/utilities.min.css"/>
    <script
      src="https://unpkg.com/@tailwindcss/browser@4"
    ></script>
    <script
      src="https://unpkg.com/franken-ui@2.0.0-internal.42/dist/js/core.iife.js"
      type="module"
    ></script>
    <script
      src="https://unpkg.com/franken-ui@2.0.0-internal.42/dist/js/icon.iife.js"
      type="module"
    ></script>
</head>
<body class="bg-background text-foreground">
    {}
</body>
</html>"#,
        title, content
    )
}

pub fn base_auth(title: &str, other_action: &str, content: &str) -> String {
    let auth_content = format!(
        r#"
<div class="min-h-screen grid xl:grid-cols-2">
    <div class="hidden xl:flex flex-col justify-between bg-zinc-900 p-8 text-white">
        <div class="flex items-center text-lg font-medium">
            ayb
        </div>
        <blockquote class="space-y-2">
            <p class="text-lg">You're a minute away from creating, sharing, and querying a database.</p>
        </blockquote>
    </div>
    <div class="flex flex-col p-8">
        <div class="flex justify-end">
            {}
        </div>
        <div class="flex flex-1 items-center justify-center">
            {}
        </div>
    </div>
</div>"#,
        other_action, content
    );

    base_template(title, &auth_content)
}

pub fn base_content(title: &str, content: &str) -> String {
    let nav = r#"
    <nav class="bg-white shadow-sm mb-6">
        <div class="max-w-4xl mx-auto px-6 py-4">
            <div class="flex justify-between items-center">
                <a href="/" class="text-xl font-bold">AYB</a>
                <div class="flex gap-4">
                    <a href="/register" class="text-gray-600 hover:text-gray-900">Register</a>
                    <a href="/login" class="text-gray-600 hover:text-gray-900">Login</a>
                </div>
            </div>
        </div>
    </nav>
    <div class="max-w-4xl mx-auto px-6">
        "#;

    let wrapped_content = format!("{}{}</div>", nav, content);
    base_template(title, &wrapped_content)
}

pub fn create_client(
    config: &crate::server::config::AybConfig,
    auth_token: Option<String>,
) -> crate::client::http::AybClient {
    crate::client::http::AybClient {
        base_url: format!("http://{}:{}", config.host, config.port),
        api_token: auth_token,
    }
}
