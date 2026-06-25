fn main() {
    #[cfg(target_os = "windows")]
    {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("assets/favicon.ico");
        res.set("ProductName", "MD -> ALL");
        res.set("FileDescription", "Markdown Editor with KaTeX and PDF Export");
        res.set("ProductVersion", "3.0.0");
        if let Err(e) = res.compile() {
            eprintln!("winresource warning: {}", e);
        }
    }

    // Warn at build time if the bundled Chromium binary is absent.
    // PDF export will still work via the Typst / genpdf fallbacks,
    // but the highest-quality tier (CDP) requires chromium/chrome.exe
    // adjacent to the executable.
    // Run scripts/setup-chromium.ps1 to download and install it.
    let chrome_candidate = std::path::Path::new("chromium").join("chrome.exe");
    if !chrome_candidate.exists() {
        println!(
            "cargo:warning=Bundled Chromium not found at chromium/chrome.exe. \
             High-quality PDF export (tier 1) will be unavailable at runtime. \
             Run: .\\scripts\\setup-chromium.ps1"
        );
    }
}
