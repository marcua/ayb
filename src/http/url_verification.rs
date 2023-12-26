use scraper::{Html, Selector};
use url::Url;

/// Verifies that the HTML website located at `input_url` contains an anchor tag whose `rel` attribute is set to `me`
/// and whose `href` attribute is equal to the value of `expected_url`.
pub async fn is_verified_url(input_url: Url, expected_url: Url) -> bool {
    if let Ok(website) = reqwest::get(input_url.to_string()).await {
        if let Ok(website) = website.text().await {
            let website = Html::parse_document(&website);
            let raw_selector = format!("a[href=\"{}\"][rel=\"me\"]", expected_url);
            let selector = Selector::parse(&raw_selector);

            return selector.is_ok_and(|selector| website.select(&selector).next().is_some());
        }
    }

    false
}
