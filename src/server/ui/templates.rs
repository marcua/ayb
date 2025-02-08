pub fn base_template(title: &str, content: &str) -> String {
    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>{} - AYB</title>
    <link rel="stylesheet" href="https://unpkg.com/franken-ui@2.0.0-internal.41/dist/css/core.min.css"/>
    <link rel="stylesheet" href="https://unpkg.com/franken-ui@2.0.0-internal.41/dist/css/utilities.min.css"/>
</head>
<body class="bg-gray-50">
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
        {}
    </div>
</body>
</html>"#,
        title, content
    )
}

pub fn create_client(config: &crate::server::config::AybConfig, auth_token: Option<String>) -> crate::client::http::AybClient {
    crate::client::http::AybClient {
        base_url: format!("http://{}:{}", config.host, config.port),
        api_token: auth_token,
    }
}
