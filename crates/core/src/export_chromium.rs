// Chromium CDP-based PDF export - uses bundled ungoogled-chromium binary
// No dependency on system browsers; fully controlled, isolated binary.

use crate::export::PdfMetadata;
use std::path::Path;

const ISOLATION_FLAGS: &[&str] = &[
    "--no-sandbox",
    "--disable-gpu",
    "--no-first-run",
    "--disable-default-apps",
    "--disable-sync",
    "--disable-translate",
    "--disable-extensions",
    "--disable-component-update",
    "--disable-background-networking",
    "--disable-client-side-phishing-detection",
    "--safebrowsing-disable-auto-update",
    "--metrics-recording-only",
    "--disable-features=ChromeWhatsNewUI,TranslateUI",
    "--no-default-browser-check",
    "--disable-popup-blocking",
];

/// Find the bundled Chromium binary shipped alongside the executable.
/// Search order:
///   1. <exe_dir>/chromium/<os binary>  - main bundle location
///   2. MD2ALL_CHROMIUM env var         - operator override
///
/// The bundled binary name differs per OS: `chrome.exe` on Windows, `chrome`
/// or `chromium` on Linux, the app bundle on macOS.
fn find_bundled_chromium() -> Option<std::path::PathBuf> {
    #[cfg(target_os = "windows")]
    let names: &[&str] = &["chrome.exe"];
    #[cfg(target_os = "macos")]
    let names: &[&str] = &[
        "Chromium.app/Contents/MacOS/Chromium",
        "Google Chrome for Testing.app/Contents/MacOS/Google Chrome for Testing",
        "chrome",
    ];
    #[cfg(all(unix, not(target_os = "macos")))]
    let names: &[&str] = &["chrome", "chromium", "chrome-wrapper"];

    // 1. Relative to executable: <exe_dir>/chromium/<name>
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            for name in names {
                let candidate = dir.join("chromium").join(name);
                if candidate.exists() {
                    return Some(candidate);
                }
            }
        }
    }

    // 2. Explicit env var (allows operator override without recompilation)
    if let Ok(val) = std::env::var("MD2ALL_CHROMIUM") {
        let p = std::path::PathBuf::from(val);
        if p.exists() {
            return Some(p);
        }
    }

    None
}

/// Synchronous entry point - wraps async CDP in a blocking Tokio runtime.
pub fn export_pdf_chromium(
    markdown: &str,
    output_path: &Path,
    metadata: &PdfMetadata,
    source_dir: Option<&Path>,
) -> Result<(), String> {
    let chrome = find_bundled_chromium()
        .ok_or_else(|| "Bundled Chromium not found (expected under <exe>/chromium/)".to_string())?;

    let rt = tokio::runtime::Runtime::new()
        .map_err(|e| format!("Tokio runtime error: {}", e))?;

    rt.block_on(run_cdp_export(markdown, output_path, metadata, source_dir, chrome))
}

async fn run_cdp_export(
    markdown: &str,
    output_path: &Path,
    metadata: &PdfMetadata,
    source_dir: Option<&Path>,
    chrome_path: std::path::PathBuf,
) -> Result<(), String> {
    use chromiumoxide::browser::{Browser, BrowserConfig};
    use chromiumoxide::cdp::browser_protocol::page::PrintToPdfParams;
    use futures_util::StreamExt;

    // Write HTML to temp file - reuses the identical pipeline as HTML export
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    let tmp_path = std::env::temp_dir().join(format!("md2all_{}.html", ts));
    crate::export::export_html(markdown, &tmp_path, metadata, source_dir)?;

    let tmp_url = format!(
        "file:///{}",
        tmp_path.display().to_string().replace('\\', "/")
    );

    // Build browser config with isolation flags
    let config = BrowserConfig::builder()
        .chrome_executable(chrome_path)
        .args(ISOLATION_FLAGS.iter().copied())
        .build()
        .map_err(|e| format!("Chromium config error: {}", e))?;

    let (mut browser, mut handler) = Browser::launch(config)
        .await
        .map_err(|e| format!("Chromium launch failed: {}", e))?;

    // Drive the handler in a background task - required for CDP to work
    tokio::spawn(async move {
        while let Some(_event) = handler.next().await {}
    });

    // Open page (navigates and waits for load event)
    let page = browser
        .new_page(&tmp_url)
        .await
        .map_err(|e| format!("Page navigation failed: {}", e))?;

    // Generate PDF with print-quality settings
    let pdf_params = PrintToPdfParams {
        print_background: Some(true),
        paper_width: Some(8.5),
        paper_height: Some(11.0),
        margin_top: Some(0.4),
        margin_bottom: Some(0.4),
        margin_left: Some(0.5),
        margin_right: Some(0.5),
        prefer_css_page_size: Some(true),
        ..Default::default()
    };

    let pdf_bytes = page
        .pdf(pdf_params)
        .await
        .map_err(|e| format!("PDF generation failed: {}", e))?;

    let _ = std::fs::remove_file(&tmp_path);

    if pdf_bytes.is_empty() {
        return Err("Chromium produced an empty PDF".to_string());
    }

    std::fs::write(output_path, &pdf_bytes)
        .map_err(|e| format!("PDF write error: {}", e))?;

    browser.close().await.ok();
    Ok(())
}
