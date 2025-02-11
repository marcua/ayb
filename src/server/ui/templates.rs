fn base_template(title: &str, content: &str) -> String {
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
    {}
</body>
</html>"#,
        title, content
    )
}

# AI! Clean this function up
# - Remove the everything from "Create account" to the "Sign in with Email" button and replace that with a {} template of the content argument
# - Move the create account form into the other location I've marked
pub fn base_auth(title: &str, content: &str) -> String {
    let auth_content = format!(r#"
<div style="display: contents">
	<!--[-->
	<!--[-->
	<!---->
	<!---->
	<div class="hidden h-screen grid-cols-2 xl:grid">
		<div class="col-span-1 hidden flex-col justify-between bg-zinc-900 p-8 text-white lg:flex">
			<div class="flex items-center text-lg font-medium">
            ayb
			</div>
			<blockquote class="space-y-2">
				<p class="text-lg">"This library has saved me countless hours of work and helped me deliver stunning designs to
				my clients faster than ever before."</p>
				<footer class="text-sm">Sofia Davis</footer>
			</blockquote>
		</div>
		<div class="col-span-2 flex flex-col p-8 lg:col-span-1">
			<div class="flex flex-none justify-end">
				<button class="uk-btn uk-btn-ghost">Login</button>
			</div>
			<div class="flex flex-1 items-center justify-center">
				<div class="w-80 space-y-6">
					<div class="flex flex-col space-y-2 text-center">
						<h1 class="uk-h3">Create an account</h1>
						<p class="text-sm text-muted-foreground">Enter your email below to create your account</p>
					</div>
					<div class="space-y-2">
						<input class="uk-input" placeholder="name@example.com" type="text">
							<button class="uk-btn uk-btn-primary w-full">
								<!--[!-->
								<!--]--> Sign in with Email
							</button>
						</div>
					</div>
				</div>
			</div>
		</div>
		<!---->
		<!---->
		<!---->
		<!--]-->
		<!--[!-->
		<!--]-->
		<!--]-->
    </div>
</div>"#, content);
    base_template(title, auth_content)
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
