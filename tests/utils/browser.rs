use ayb::email::backend::EmailEntry as AybEmailEntry;
use image::GenericImageView;
use playwright::{api::Page, Playwright};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

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
    /// Parse email file (JSONL format)
    pub fn parse_email_file(
        file_path: &str,
    ) -> Result<Vec<AybEmailEntry>, Box<dyn std::error::Error>> {
        let content = std::fs::read_to_string(file_path)?;
        let mut emails = Vec::new();

        for line in content.lines() {
            let line = line.trim();
            if !line.is_empty() {
                emails.push(serde_json::from_str(line)?);
            }
        }

        Ok(emails)
    }

    /// Extract token from emails
    pub fn extract_token_from_emails(emails: &[AybEmailEntry]) -> Option<String> {
        for email in emails {
            for line in &email.content {
                if line.starts_with('\t') {
                    if let Some(token_part) = line.split("ayb client confirm ").nth(1) {
                        return Some(token_part.trim().to_string());
                    }
                }
            }
        }
        None
    }
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

    /// Take a screenshot and compare it to a stored reference image
    /// Optionally grey out elements that should be ignored in comparison
    ///
    /// # Arguments
    /// * `page` - The Playwright page to screenshot
    /// * `test_name` - Name for the test (used for file naming)
    /// * `selectors_to_grey` - CSS selectors for elements to grey out before comparison
    ///
    /// # Returns
    /// * `Ok(true)` - Screenshots match (or reference was created)
    /// * `Ok(false)` - Screenshots differ significantly
    /// * `Err(...)` - Error taking screenshot or processing images
    ///
    /// # Example
    /// ```ignore
    /// let matches = BrowserHelpers::screenshot_compare(
    ///     &page,
    ///     "login_page",
    ///     &[".timestamp", ".session-id"]
    /// ).await?;
    /// ```
    pub async fn screenshot_compare(
        page: &Page,
        test_name: &str,
        selectors_to_grey: &[&str],
    ) -> Result<bool, Box<dyn std::error::Error>> {
        let screenshots_dir = "tests/screenshots";
        std::fs::create_dir_all(screenshots_dir)?;

        let reference_path = format!("{}/{}_reference.png", screenshots_dir, test_name);
        let current_path = format!("{}/{}_current.png", screenshots_dir, test_name);
        let diff_path = format!("{}/{}_diff.png", screenshots_dir, test_name);

        // Grey out specified elements before taking screenshot
        for selector in selectors_to_grey {
            let script = format!(
                r#"
                document.querySelectorAll('{}').forEach(el => {{
                    el.style.background = '#f0f0f0';
                    el.style.color = '#888888';
                    el.style.borderColor = '#cccccc';
                }});
                "#,
                selector
            );
            if let Err(e) = page
                .evaluate::<serde_json::Value, serde_json::Value>(&script, serde_json::Value::Null)
                .await
            {
                println!("Warning: Could not grey out selector '{}': {}", selector, e);
            }
        }

        // Take current screenshot
        page.screenshot_builder()
            .path(PathBuf::from(&current_path))
            .full_page(true)
            .screenshot()
            .await?;

        // If no reference exists, save current as reference
        if !Path::new(&reference_path).exists() {
            std::fs::copy(&current_path, &reference_path)?;
            println!("Created reference screenshot: {}", reference_path);
            return Ok(true);
        }

        // Compare images
        let current_img = image::open(&current_path)?;
        let reference_img = image::open(&reference_path)?;

        if current_img.dimensions() != reference_img.dimensions() {
            println!(
                "Screenshot dimensions differ: current {:?} vs reference {:?}",
                current_img.dimensions(),
                reference_img.dimensions()
            );
            return Ok(false);
        }

        let current_rgba = current_img.to_rgba8();
        let reference_rgba = reference_img.to_rgba8();

        let mut diff_pixels = 0u32;

        for (current_pixel, reference_pixel) in current_rgba.pixels().zip(reference_rgba.pixels()) {
            let diff = ((current_pixel[0] as i32 - reference_pixel[0] as i32).abs()
                + (current_pixel[1] as i32 - reference_pixel[1] as i32).abs()
                + (current_pixel[2] as i32 - reference_pixel[2] as i32).abs())
                as u32;

            if diff > 30 {
                // Threshold for significant difference
                diff_pixels += 1;
            }
        }

        let total_pixels = current_rgba.pixels().len() as u32;
        let diff_percentage = (diff_pixels as f64 / total_pixels as f64) * 100.0;

        if diff_percentage > 5.0 {
            // More than 5% different pixels
            // Create diff image highlighting differences in red
            let mut diff_buffer = current_rgba.clone();
            for (i, (current_pixel, reference_pixel)) in current_rgba
                .pixels()
                .zip(reference_rgba.pixels())
                .enumerate()
            {
                let diff = ((current_pixel[0] as i32 - reference_pixel[0] as i32).abs()
                    + (current_pixel[1] as i32 - reference_pixel[1] as i32).abs()
                    + (current_pixel[2] as i32 - reference_pixel[2] as i32).abs())
                    as u32;

                if diff > 30 {
                    let y = i as u32 / current_rgba.width();
                    let x = i as u32 % current_rgba.width();
                    diff_buffer.put_pixel(x, y, image::Rgba([255, 0, 0, 255])); // Red
                }
            }

            // Save diff image
            diff_buffer.save(&diff_path)?;
            println!(
                "Screenshots differ by {:.2}% - diff saved to {}",
                diff_percentage, diff_path
            );
            Ok(false)
        } else {
            println!("Screenshots match (difference: {:.2}%)", diff_percentage);
            // Clean up current screenshot if it matches
            let _ = std::fs::remove_file(&current_path);
            Ok(true)
        }
    }
}
