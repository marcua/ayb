use playwright::{api::Page, Playwright};
use serde::{Deserialize, Serialize};
use std::fs;
use std::time::Duration;

#[derive(Serialize, Deserialize)]
pub struct EmailEntry {
    pub from: String,
    pub to: String,
    pub reply_to: String,
    pub subject: String,
    pub content_type: String,
    pub content_transfer_encoding: String,
    pub date: String,
    pub content: Vec<String>,
}

pub struct BrowserHelpers;

impl BrowserHelpers {
    /// Initialize playwright and return browser page
    pub async fn setup_browser() -> Result<(Playwright, Page), Box<dyn std::error::Error>> {
        use std::path::Path;

        let playwright = Playwright::initialize().await?;
        // Skip playwright.prepare() - don't install browsers, use system ones

        // Check for BROWSER_VISIBLE environment variable to run in non-headless mode
        let headless = std::env::var("BROWSER_VISIBLE").is_err();

        if !headless {
            println!("🌐 Running browser in VISIBLE mode for debugging");
        }

        // Use system Chrome - confirmed working on Mac M1 ARM64
        let chromium = playwright.chromium();
        let browser = chromium
            .launcher()
            .headless(headless)
            .executable(Path::new(
                "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
            ))
            .launch()
            .await?;
        let context = browser.context_builder().build().await?;
        let page = context.new_page().await?;

        Ok((playwright, page))
    }

    /// Navigate to a page and wait for it to load
    pub async fn navigate_and_wait(
        page: &Page,
        url: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Much faster navigation with shorter timeout
        page.goto_builder(url)
            .timeout(5000.0) // 5 second timeout
            .goto()
            .await?;

        // Minimal wait for page to stabilize
        tokio::time::sleep(Duration::from_millis(100)).await;

        Ok(())
    }

    /// Click an element if it exists
    pub async fn click_if_exists(
        page: &Page,
        selector: &str,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        match page.click_builder(selector).timeout(1000.0).click().await {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }

    /// Fill a form field
    pub async fn fill_field_if_exists(
        page: &Page,
        selector: &str,
        value: &str,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        match page
            .fill_builder(selector, value)
            .timeout(1000.0)
            .fill()
            .await
        {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }

    /// Check if an element exists by trying to find error-related elements
    pub async fn has_error_message(page: &Page) -> Result<bool, Box<dyn std::error::Error>> {
        // Use a single selector that covers most common error patterns
        let error_selector = ".error, .alert-danger, .alert-error, .invalid-feedback, .field-error";

        if let Ok(elements) = page.query_selector_all(error_selector).await {
            Ok(!elements.is_empty())
        } else {
            Ok(false)
        }
    }

    /// Wait for page to load and check if it contains expected content
    pub async fn wait_for_page_content(
        page: &Page,
        expected_content: &str,
        timeout_ms: u32,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        let start_time = std::time::Instant::now();

        while start_time.elapsed().as_millis() < timeout_ms as u128 {
            if let Ok(content) = page.content().await {
                if content
                    .to_lowercase()
                    .contains(&expected_content.to_lowercase())
                {
                    return Ok(true);
                }
            }
            tokio::time::sleep(Duration::from_millis(25)).await;
        }

        Ok(false)
    }

    /// Fill a registration form with provided data (ayb uses passwordless magic links)
    pub async fn fill_registration_form(
        page: &Page,
        email: &str,
        username: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Fill email field - use most specific selector first
        Self::fill_field_if_exists(page, "input[name='email']", email).await?;

        // Fill username field - use most specific selector first
        Self::fill_field_if_exists(page, "input[name='username']", username).await?;

        Ok(())
    }

    /// Fill a login form with username (ayb uses passwordless magic links)
    pub async fn fill_login_form(
        page: &Page,
        username: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Fill username field - use most specific selector
        Self::fill_field_if_exists(page, "input[name='username']", username).await?;
        Ok(())
    }

    /// Submit a form by clicking the submit button
    pub async fn submit_form(page: &Page) -> Result<bool, Box<dyn std::error::Error>> {
        // Use single selector that covers most submit button patterns
        Self::click_if_exists(page, "input[type='submit'], button[type='submit'], button").await
    }

    /// Wait for navigation after form submission
    pub async fn wait_for_navigation_or_error(
        page: &Page,
        timeout_ms: u32,
    ) -> Result<NavigationResult, Box<dyn std::error::Error>> {
        let start_time = std::time::Instant::now();
        let initial_url = page.url()?;

        while start_time.elapsed().as_millis() < timeout_ms as u128 {
            // Check if URL changed (navigation occurred)
            let current_url = page.url()?;
            if current_url != initial_url {
                return Ok(NavigationResult::NavigationOccurred);
            }

            // Check if error message appeared
            if Self::has_error_message(page).await? {
                return Ok(NavigationResult::ErrorDisplayed);
            }

            tokio::time::sleep(Duration::from_millis(25)).await;
        }

        Ok(NavigationResult::Timeout)
    }

    /// Parse SMTP log file to get captured emails
    pub fn parse_smtp_log(file_path: &str) -> Result<Vec<EmailEntry>, Box<dyn std::error::Error>> {
        let mut entries = Vec::new();
        if !std::path::Path::new(file_path).exists() {
            return Err(format!("SMTP log file not found: {}", file_path).into());
        }

        for line in fs::read_to_string(file_path)?.lines() {
            if !line.trim().is_empty() {
                entries.push(serde_json::from_str(line)?);
            }
        }
        Ok(entries)
    }

    /// Extract confirmation token from email content
    pub fn extract_token(email: &EmailEntry) -> Result<String, Box<dyn std::error::Error>> {
        // Look for URL format: http://localhost:5435/confirm/{token}
        for line in &email.content {
            if line.contains("/confirm/") {
                if let Some(start_pos) = line.find("/confirm/") {
                    let token_start = start_pos + "/confirm/".len();
                    if token_start < line.len() {
                        let token = line[token_start..].trim();
                        if !token.is_empty() {
                            return Ok(token.to_string());
                        }
                    }
                }
            }
        }

        // Fallback: Look for older CLI format
        let prefix = "\tayb client confirm ";
        for line in &email.content {
            if line.starts_with(prefix) && line.len() > prefix.len() {
                let token_bytes = quoted_printable::decode(
                    &line[prefix.len()..],
                    quoted_printable::ParseMode::Robust,
                )?;
                return Ok(String::from_utf8(token_bytes)?);
            }
        }

        Err("No confirmation token found in email".into())
    }

    /// Complete registration by navigating to confirmation URL
    pub async fn confirm_registration(
        page: &Page,
        server_url: &str,
        smtp_port: u16,
        email: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let start_time = std::time::Instant::now();
        println!("    Waiting for confirmation email...");

        // Wait for email to be received and try multiple times
        let smtp_log_path = format!("tests/smtp_data_{}/{}", smtp_port, email);
        let mut emails = Vec::new();

        for attempt in 1..=10 {
            tokio::time::sleep(Duration::from_millis(500)).await;

            match Self::parse_smtp_log(&smtp_log_path) {
                Ok(parsed_emails) => {
                    if !parsed_emails.is_empty() {
                        emails = parsed_emails;
                        println!(
                            "    ✓ Found {} email(s) after {:.2}s",
                            emails.len(),
                            start_time.elapsed().as_secs_f64()
                        );
                        break;
                    }
                }
                Err(e) if attempt < 10 => {
                    println!("    Attempt {}: {}", attempt, e);
                    continue;
                }
                Err(e) => return Err(e),
            }
        }

        if emails.is_empty() {
            return Err("No confirmation email received after 5 seconds".into());
        }

        // Extract token from the latest email
        let latest_email = emails.last().unwrap();
        println!("    Latest email subject: '{}'", latest_email.subject);

        let token = Self::extract_token(latest_email)?;
        println!(
            "    ✓ Extracted confirmation token (length: {})",
            token.len()
        );

        // Navigate to confirmation URL
        let confirm_url = format!("{}/confirm/{}", server_url, token);
        println!("    Navigating to confirmation URL: {}", confirm_url);

        Self::navigate_and_wait(page, &confirm_url).await?;

        println!(
            "    ✓ Registration confirmed successfully (took {:.2}s)",
            start_time.elapsed().as_secs_f64()
        );
        Ok(())
    }

    /// Verify query results contain the complete expected dataset
    pub async fn verify_query_results(
        page: &Page,
        expected_datasets: &[(&str, &str, &str)], // (name, description, url) tuples
        timeout_ms: u32,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        let start_time = std::time::Instant::now();

        while start_time.elapsed().as_millis() < timeout_ms as u128 {
            if let Ok(content) = page.content().await {
                let content_lower = content.to_lowercase();

                // Check if all expected datasets are present
                let mut all_found = true;
                for (name, description, url) in expected_datasets {
                    if !content_lower.contains(&name.to_lowercase())
                        || !content_lower.contains(&description.to_lowercase())
                        || !content_lower.contains(&url.to_lowercase())
                    {
                        all_found = false;
                        break;
                    }
                }

                if all_found {
                    return Ok(true);
                }
            }
            tokio::time::sleep(Duration::from_millis(25)).await;
        }

        Ok(false)
    }

    /// Find and read the most recent download file with given extension
    pub async fn find_and_read_download(
        downloads_dir: &str,
        extension: &str,
    ) -> Result<String, Box<dyn std::error::Error>> {
        use std::fs;
        use std::time::SystemTime;

        let mut latest_file: Option<(std::path::PathBuf, SystemTime)> = None;

        // Read the downloads directory
        if let Ok(entries) = fs::read_dir(downloads_dir) {
            for entry in entries {
                if let Ok(entry) = entry {
                    let path = entry.path();
                    if let Some(file_extension) = path.extension() {
                        if file_extension == extension {
                            if let Ok(metadata) = entry.metadata() {
                                if let Ok(modified) = metadata.modified() {
                                    match &latest_file {
                                        None => latest_file = Some((path, modified)),
                                        Some((_, latest_time)) => {
                                            if modified > *latest_time {
                                                latest_file = Some((path, modified));
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        if let Some((file_path, _)) = latest_file {
            let content = fs::read_to_string(&file_path)?;
            println!("    Found download file: {}", file_path.display());
            Ok(content)
        } else {
            Err(format!("No {} file found in {}", extension, downloads_dir).into())
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum NavigationResult {
    NavigationOccurred,
    ErrorDisplayed,
    Timeout,
}
