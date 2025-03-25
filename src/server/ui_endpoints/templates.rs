fn base_template(title: &str, content: &str, redirect: Option<String>) -> String {
    format!(
        r#"<!DOCTYPE html>
<html
  lang="en"
  class="uk-theme-blue uk-radii-md uk-shadows-md uk-font-sm">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    {}
    <title>{} - ayb</title>
    <link rel="preconnect" href="https://fonts.googleapis.com">
    <link rel="preconnect" href="https://fonts.gstatic.com" crossorigin>
    <link href="https://fonts.googleapis.com/css2?family=Geist:wght@100..900&display=swap" rel="stylesheet">
    <style>
      :root {{
          font-family: "Geist";
          font-optical-sizing: auto;
          font-style: normal;
      }}
    </style>

    <link
        rel="stylesheet"
        href="https://unpkg.com/franken-ui@2.0.0/dist/css/core.min.css"
    />
    <link
        rel="stylesheet"
        href="https://unpkg.com/franken-ui@2.0.0/dist/css/utilities.min.css"
    />
    <script
      src="https://unpkg.com/franken-ui@2.0.0-internal.42/dist/js/core.iife.js"
      type="module"
    ></script>
    <script
      src="https://unpkg.com/franken-ui@2.0.0-internal.42/dist/js/icon.iife.js"
      type="module"
    ></script>
    <!-- TODO(marcua): does it need type="module"? Can we only include it where necessary, like on the database page? -->
    <script
      src="https://unpkg.com/htmx.org@2.0.4">
    </script>
</head>
<body class="bg-background text-foreground">
    {}
</body>
</html>"#,
        redirect.as_ref().map_or(String::new(), |url| format!(
            r#"<meta http-equiv="refresh" content="0; url={}" />"#,
            url
        )),
        title,
        content
    )
}

pub fn base_auth(
    title: &str,
    other_action: &str,
    content: &str,
    redirect: Option<String>,
) -> String {
    let auth_content = format!(
        r#"
<div class="min-h-screen grid xl:grid-cols-2">
    <div class="hidden xl:flex flex-col justify-between bg-foreground text-background p-8">
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

    base_template(title, &auth_content, redirect)
}

pub fn base_content(title: &str, content: &str) -> String {
    // TODO(marcua): Log out (username) vs Log in, Register
    let nav = r#"
    <nav class="bg-white shadow-sm mb-6">
        <div class="max-w-screen-xl mx-auto px-6 py-4">
            <div class="flex justify-between items-center">
                <a href="/" class="text-xl font-bold">ayb</a>
                <div class="flex gap-4">
                    <a href="/register" class="text-gray-600 hover:text-gray-900">Register</a>
                    <a href="/log_in" class="text-gray-600 hover:text-gray-900">Log in</a>
                </div>
            </div>
        </div>
    </nav>
    <div class="max-w-screen-xl mx-auto px-6">
        "#;

    let wrapped_content = format!("{}{}</div>", nav, content);
    base_template(title, &wrapped_content, None)
}
