use actix_web::{get, http::header, web, HttpResponse};

const CSS: &str = "text/css; charset=utf-8";
const JS: &str = "text/javascript; charset=utf-8";

/// Third-party UI assets, embedded in the binary so pages don't depend
/// on a CDN at runtime. See static/README.md for where each came from.
const STATIC_FILES: &[(&str, &str, &[u8])] = &[
    (
        "franken-ui-2.0.0/core.min.css",
        CSS,
        include_bytes!("static/franken-ui-2.0.0/core.min.css"),
    ),
    (
        "franken-ui-2.0.0/utilities.min.css",
        CSS,
        include_bytes!("static/franken-ui-2.0.0/utilities.min.css"),
    ),
    (
        "franken-ui-2.0.0-internal.42/core.iife.js",
        JS,
        include_bytes!("static/franken-ui-2.0.0-internal.42/core.iife.js"),
    ),
    (
        "franken-ui-2.0.0-internal.42/icon.iife.js",
        JS,
        include_bytes!("static/franken-ui-2.0.0-internal.42/icon.iife.js"),
    ),
    (
        "htmx-2.0.4/htmx.min.js",
        JS,
        include_bytes!("static/htmx-2.0.4/htmx.min.js"),
    ),
    (
        "htmx-ext-response-targets-2.0.3/response-targets.min.js",
        JS,
        include_bytes!("static/htmx-ext-response-targets-2.0.3/response-targets.min.js"),
    ),
];

#[get("/static/{path:.*}")]
pub async fn static_file(path: web::Path<String>) -> HttpResponse {
    match STATIC_FILES
        .iter()
        .find(|(name, _, _)| *name == path.as_str())
    {
        Some((_, content_type, body)) => HttpResponse::Ok()
            .content_type(*content_type)
            // Each path names its package version, so its contents never
            // change and browsers can cache it indefinitely.
            .insert_header((header::CACHE_CONTROL, "public, max-age=31536000, immutable"))
            .body(*body),
        None => HttpResponse::NotFound().finish(),
    }
}

#[cfg(test)]
mod tests {
    use super::static_file;
    use actix_web::{http::header, test, App};

    #[actix_web::test]
    async fn serves_static_files() {
        let app = test::init_service(App::new().service(static_file)).await;

        let req = test::TestRequest::get()
            .uri("/static/htmx-2.0.4/htmx.min.js")
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 200);
        assert_eq!(
            resp.headers().get(header::CONTENT_TYPE).unwrap(),
            "text/javascript; charset=utf-8"
        );
        assert!(!test::read_body(resp).await.is_empty());

        for missing in ["/static/nope.js", "/static/../Cargo.toml"] {
            let req = test::TestRequest::get().uri(missing).to_request();
            let resp = test::call_service(&app, req).await;
            assert_eq!(resp.status(), 404, "{missing}");
        }
    }
}
