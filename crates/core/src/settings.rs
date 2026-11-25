//! User settings, persisted as JSON in the OS config folder under the Core's home:
//! `~/Library/Application Support/skills-pad/settings.json` on macOS,
//! `~/AppData/Roaming/skills-pad/settings.json` on Windows and
//! `~/.config/skills-pad/settings.json` elsewhere.
//!
//! Deriving the folder from `home` keeps tests (which use a temp home) isolated without extra
//! configuration. Unknown fields are kept when saving, so other parts of the app can store
//! their own settings in the same file.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

/// Settings the Core uses. Missing fields take their defaults.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// Press Enter after every paste so a tap runs the skill. Off by default.
    pub send_enter: bool,
}

/// The Skills Pad config folder for `home` on this OS.
pub fn config_dir(home: &Path) -> PathBuf {
    let base = if cfg!(target_os = "macos") {
        home.join("Library/Application Support")
    } else if cfg!(windows) {
        home.join("AppData/Roaming")
    } else {
        home.join(".config")
    };
    base.join("skills-pad")
}

pub(crate) fn settings_path(home: &Path) -> PathBuf {
    config_dir(home).join("settings.json")
}

/// Loads settings, falling back to defaults when the file is missing or unreadable.
pub(crate) fn load(home: &Path) -> Settings {
    fs::read_to_string(settings_path(home))
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default()
}

/// Saves settings, keeping any other fields already in the file.
pub(crate) fn save(home: &Path, settings: &Settings) -> io::Result<()> {
    let path = settings_path(home);
    let mut fields: Map<String, Value> = fs::read_to_string(&path)
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default();
    if let Value::Object(ours) = serde_json::to_value(settings).map_err(io::Error::other)? {
        fields.extend(ours);
    }
    fs::create_dir_all(path.parent().expect("settings path has a parent"))?;
    let tmp = path.with_extension("json.tmp");
    fs::write(
        &tmp,
        serde_json::to_vec_pretty(&fields).map_err(io::Error::other)?,
    )?;
    fs::rename(tmp, path)
}
