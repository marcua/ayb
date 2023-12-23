use html_parser::Dom;
use std::str::FromStr;
use url::Url;

pub async fn is_verified(input_url: Url, expected_url: Url) -> bool {
    let website = reqwest::get(input_url.to_string()).await;
    if website.is_err() {
        return false;
    }

    let website = website.unwrap().text().await;
    if website.is_err() {
        return false;
    }

    let website = Dom::parse(&website.unwrap());
    if website.is_err() {
        return false;
    }

    let website = website.unwrap();
    for children in website.children {
        if let Some(element) = children.element() {
            let rel = element.attributes.get("rel");
            let href = element.attributes.get("href");
            if &element.name.to_lowercase() == "a"
                && rel.is_some_and(|v| v.as_ref().is_some_and(|v| &v.to_lowercase() == "me"))
                && href.is_some_and(|v| {
                    v.as_ref()
                        .is_some_and(|v| Url::from_str(v).is_ok_and(|v| v == expected_url))
                })
            {
                return true;
            }
        }
    }

    false
}
