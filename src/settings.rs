//! The app's own settings — the things that belong to the APP, not to the
//! career-ops data root.
//!
//! Which CLI to drive and which theme to use are preferences of this client.
//! They live in the user's config directory, not in `config/profile.yml`, so
//! the native client and the web app never fight over one file.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    Dark,
    Light,
}

impl Default for Theme {
    fn default() -> Self {
        Theme::Dark
    }
}

/// The zoom used on a first run.
///
/// 1.15 rather than 1.0: egui's own default body size is 14 points, which on a
/// 2560x1440 panel is barely legible. The token scale in `theme.rs` already
/// raises that to 16.5, and this lands body copy near 19 points physical —
/// comfortable for a tracker read in long stretches. Ctrl+0 returns here,
/// Ctrl+/- moves, and the value is remembered.
pub const DEFAULT_ZOOM: f32 = 1.15;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Settings {
    /// Which CLI the run screens drive.
    pub cli_id: String,
    pub theme: Theme,
    /// How large everything is drawn. See DEFAULT_ZOOM.
    pub zoom: f32,
    /// The data root to use, when it is not the checkout's own.
    pub data_root: String,
    /// The postings shortlisted for a batch scoring run.
    ///
    /// The client's own list, so it survives a restart: a shortlist you cannot
    /// come back to is a shortlist you have to rebuild.
    pub shortlist: Vec<String>,
    /// The screen the app was on when it was last closed.
    ///
    /// Reopening lands where you left off, which is the whole of E16-S08.
    pub last_screen: String,
    /// The career-ops checkout this client drives.
    ///
    /// The engine lives in its own repository, so the checkout is a choice the
    /// user makes rather than a fact the app infers. It is remembered here so
    /// the choice survives a restart: `CAREER_OPS_CODE_ROOT` still wins when it
    /// is set, and this is the value used when it is not.
    pub code_root: String,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            cli_id: String::new(),
            theme: Theme::Dark,
            zoom: DEFAULT_ZOOM,
            data_root: String::new(),
            code_root: String::new(),
            last_screen: String::new(),
            shortlist: Vec::new(),
        }
    }
}

impl Settings {
    /// Where the settings live.
    ///
    /// `$XDG_CONFIG_HOME/career-ops-desktop/settings.json`, falling back to
    /// `$HOME/.config/...`. A missing file is not an error: the defaults are
    /// the settings.
    pub fn path() -> PathBuf {
        let base = std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .filter(|p| !p.as_os_str().is_empty())
            .or_else(|| {
                std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config"))
            })
            .unwrap_or_else(|| PathBuf::from(".config"));
        base.join("career-ops-desktop").join("settings.json")
    }

    /// Read the settings, falling back to the defaults.
    pub fn load() -> Settings {
        let path = Settings::path();
        std::fs::read_to_string(path)
            .ok()
            .and_then(|text| serde_json::from_str(&text).ok())
            .unwrap_or_default()
    }

    /// Write the settings. A failure is not fatal: the app keeps running with
    /// the values it has.
    pub fn save(&self) -> Result<(), String> {
        let path = Settings::path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("could not create {}: {e}", parent.display()))?;
        }
        let body = serde_json::to_string_pretty(self)
            .map_err(|e| format!("could not serialize the settings: {e}"))?;
        std::fs::write(&path, body)
            .map_err(|e| format!("could not write {}: {e}", path.display()))
    }
}
