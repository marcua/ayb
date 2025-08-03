use image::GenericImageView;
use playwright::{api::Page, Playwright};
use std::path::{Path, PathBuf};

pub struct BrowserHelpers;

impl BrowserHelpers {
    /// Initialize playwright and return browser page
    pub async fn setup_browser() -> Result<(Playwright, Page), Box<dyn std::error::Error>> {
        let playwright = Playwright::initialize().await?;

        // Check for BROWSER_VISIBLE environment variable to run in non-headless mode
        let headless = std::env::var("BROWSER_VISIBLE").is_err();

        if !headless {
            println!("🌐 Running browser in VISIBLE mode for debugging");
        }

        let chromium = playwright.chromium();

        // Try different browser launch strategies in order of preference
        let browser = Self::try_launch_browser(&chromium, headless).await?;

        let context = browser.context_builder().build().await?;
        let page = context.new_page().await?;

        Ok((playwright, page))
    }

    async fn try_launch_browser(
        chromium: &playwright::api::BrowserType,
        headless: bool,
    ) -> Result<playwright::api::Browser, Box<dyn std::error::Error>> {
        // Strategy 1: Try Playwright-installed browser first
        match chromium.launcher().headless(headless).launch().await {
            Ok(browser) => return Ok(browser),
            Err(_) => println!("Playwright installation not possible on this platform, trying fallbacks..."),
        }

        // Strategy 2: Try platform-specific system browsers
        let browser_paths = if cfg!(target_os = "macos") {
            vec![
                "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
                "/Applications/Chromium.app/Contents/MacOS/Chromium",
            ]
        } else if cfg!(target_os = "linux") {
            vec![
                "/usr/bin/google-chrome",
                "/usr/bin/google-chrome-stable",
                "/usr/bin/chromium-browser",
                "/usr/bin/chromium",
            ]
        } else {
            vec![]
        };

        for path in &browser_paths {
            if std::path::Path::new(path).exists() {
                println!("Trying system browser at: {}", path);
                match chromium
                    .launcher()
                    .headless(headless)
                    .executable(std::path::Path::new(path))
                    .launch()
                    .await
                {
                    Ok(browser) => return Ok(browser),
                    Err(_) => {}, // Try next fallback
                }
            }
        }

        Err("No working browser found. Tried Playwright-installed and system browsers.".into())
    }

    /// Take a screenshot and compare it to a stored reference image
    /// Optionally grey out elements that should be ignored in comparison
    /// Automatically prints comparison results with standardized messages
    ///
    /// # Arguments
    /// * `page` - The Playwright page to screenshot
    /// * `test_name` - Name for the test (used for file naming and messages)
    /// * `selectors_to_grey` - CSS selectors for elements to grey out before comparison
    ///
    /// # Returns
    /// * `Ok(())` - Screenshot comparison completed (prints result automatically)
    /// * `Err(...)` - Error taking screenshot or processing images
    ///
    /// # Example
    /// ```ignore
    /// BrowserHelpers::screenshot_compare(
    ///     &page,
    ///     "login_page",
    ///     &[".timestamp", ".session-id"]
    /// ).await?;
    /// ```
    pub async fn screenshot_compare(
        page: &Page,
        test_name: &str,
        selectors_to_grey: &[&str],
    ) -> Result<(), Box<dyn std::error::Error>> {
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
            println!("📸 Created reference screenshot for '{}'", test_name);
            return Ok(());
        }

        // Compare images
        let current_img = image::open(&current_path)?;
        let reference_img = image::open(&reference_path)?;

        if current_img.dimensions() != reference_img.dimensions() {
            let error_msg = format!(
                "Screenshot '{}' dimensions differ: current {:?} vs reference {:?}",
                test_name,
                current_img.dimensions(),
                reference_img.dimensions()
            );
            println!("⚠ {}", error_msg);
            return Err(error_msg.into());
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
            let error_msg = format!(
                "Screenshot '{}' differs from reference by {:.2}% - diff saved to {}",
                test_name, diff_percentage, diff_path
            );
            println!("⚠ {}", error_msg);
            Err(error_msg.into())
        } else {
            println!(
                "✓ Screenshot '{}' matches reference (difference: {:.2}%)",
                test_name, diff_percentage
            );
            // Clean up current screenshot if it matches
            let _ = std::fs::remove_file(&current_path);
            Ok(())
        }
    }
}
