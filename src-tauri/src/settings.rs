use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InsertionMode {
    /// Text is typed word-by-word as the user speaks into the focused input.
    Live,
    /// Text is typed all at once when the shortcut is released.
    OnRelease,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AppSettings {
    /// Active model identifier (see models::CATALOG).
    pub model_id: String,
    /// BCP-47 language code ("fr", "en", …) or None for automatic detection.
    pub language: Option<String>,
    /// Push-to-talk shortcut accelerator, e.g. "Ctrl+Alt+Space".
    pub hotkey: String,
    pub insertion_mode: InsertionMode,
    /// Keep a local history of transcriptions.
    pub history_enabled: bool,
    /// Launch Echo at login.
    pub autostart: bool,
    /// UI language: "fr", "en", or None to follow the system.
    pub ui_language: Option<String>,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            model_id: "parakeet-tdt-0.6b-v3".to_string(),
            language: None,
            hotkey: default_hotkey().to_string(),
            insertion_mode: InsertionMode::OnRelease,
            history_enabled: true,
            autostart: false,
            ui_language: None,
        }
    }
}

fn default_hotkey() -> &'static str {
    #[cfg(target_os = "windows")]
    {
        "Ctrl+Shift+Space"
    }
    #[cfg(target_os = "macos")]
    {
        "Ctrl+Alt+Space"
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        "Ctrl+Alt+Space"
    }
}

/// Supported UI locales (English fallback).
pub const SUPPORTED_LOCALES: [&str; 2] = ["en", "fr"];

/// Resolves the UI language: explicit supported preference wins, then the system
/// language, then English.
pub fn resolve_locale(settings: &AppSettings) -> &'static str {
    let pick = |code: &str| -> Option<&'static str> {
        let lang = code.split(['-', '_']).next().unwrap_or("").to_lowercase();
        SUPPORTED_LOCALES.into_iter().find(|&l| l == lang)
    };
    settings
        .ui_language
        .as_deref()
        .and_then(pick)
        .or_else(|| sys_locale::get_locale().as_deref().and_then(pick))
        .unwrap_or("en")
}

fn settings_path(app: &AppHandle) -> anyhow::Result<PathBuf> {
    let dir = app.path().app_config_dir()?;
    fs::create_dir_all(&dir)?;
    Ok(dir.join("settings.json"))
}

pub fn load(app: &AppHandle) -> AppSettings {
    let mut settings: AppSettings = settings_path(app)
        .and_then(|p| Ok(fs::read_to_string(p)?))
        .ok()
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or_default();
    // The configured model may have been removed from the catalog.
    if crate::models::spec(&settings.model_id).is_none() {
        settings.model_id = AppSettings::default().model_id;
    }
    settings
}

pub fn save(app: &AppHandle, settings: &AppSettings) -> anyhow::Result<()> {
    let path = settings_path(app)?;
    fs::write(path, serde_json::to_string_pretty(settings)?)?;
    Ok(())
}
