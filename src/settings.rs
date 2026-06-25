//! Persisted user preferences (UI language, theme). Stored as JSON under
//! `%APPDATA%/MD-ALL/settings.json` (falls back next to the executable). Loaded
//! once at startup; saved automatically when a preference changes.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub struct Settings {
    pub app_lang: String,
    pub dark_mode: bool,
    /// PDF engine: true = Native converter (pure-Rust), false = General converter.
    #[serde(default)]
    pub pdf_native: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self { app_lang: "en".into(), dark_mode: false, pdf_native: false }
    }
}

fn settings_path() -> Option<PathBuf> {
    let base = std::env::var_os("APPDATA")
        .map(PathBuf::from)
        .or_else(|| {
            std::env::current_exe()
                .ok()
                .and_then(|p| p.parent().map(|d| d.to_path_buf()))
        })?;
    Some(base.join("MD-ALL").join("settings.json"))
}

/// Load saved preferences, or defaults if absent/corrupt.
pub fn load() -> Settings {
    settings_path()
        .and_then(|p| std::fs::read_to_string(p).ok())
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

/// Persist preferences (best effort; failures are silent - prefs are not critical).
pub fn save(s: &Settings) {
    let Some(path) = settings_path() else { return };
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    if let Ok(json) = serde_json::to_string_pretty(s) {
        let _ = std::fs::write(path, json);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_through_json() {
        let s = Settings { app_lang: "fr".into(), dark_mode: true, pdf_native: true };
        let json = serde_json::to_string(&s).unwrap();
        let back: Settings = serde_json::from_str(&json).unwrap();
        assert_eq!(s, back);
    }

    #[test]
    fn default_is_english_light() {
        let d = Settings::default();
        assert_eq!(d.app_lang, "en");
        assert!(!d.dark_mode);
    }
}
