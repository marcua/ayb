use crate::server::config::AybConfig;

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
    <div class="max-w-4xl mx-auto p-6">
        {}
    </div>
</body>
</html>"#,
        title, content
    )
}

pub fn card_template(content: &str) -> String {
    format!(
        r#"<div class="bg-white rounded-lg shadow-sm p-6 mb-6">
    {}
</div>"#,
        content
    )
}
