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
    /// Known Phones, with their names and Views.
    pub phones: Vec<crate::phones::PhoneRecord>,
    /// Skill Directories the user added, each assigned to a Harness.
    pub directories: Vec<crate::skills::AddedDirectory>,
    /// Harnesses the user defined.
    pub custom_harnesses: Vec<crate::harness::CustomHarness>,
    /// Invocation template overrides for registry Harnesses, by Harness ID.
    pub templates: std::collections::BTreeMap<String, String>,
    /// The network address to share on instead of the automatically chosen one.
    pub network_override: Option<std::net::IpAddr>,
    /// Saved arrangements of every Phone's View.
    pub profiles: Vec<crate::profiles::Profile>,
    /// The Profile last saved or switched to.
    pub active_profile: Option<String>,
    /// Project-relative ticket pattern. None uses the Core default.
    pub ticket_pattern: Option<String>,
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
pub fn load(home: &Path) -> Settings {
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

/// Why a settings change was refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SettingsError {
    /// The path is not an existing absolute folder.
    NotAFolder,
    /// The folder is already a Skill Directory.
    AlreadyAdded,
    UnknownHarness,
    UnknownDirectory,
    /// A custom Harness needs a name.
    EmptyName,
    /// A template must contain `{name}`.
    TemplateWithoutName,
    /// Skill Directories are still assigned to this Harness.
    HarnessInUse,
    InvalidTicketPattern,
    /// Saving the settings file failed.
    Io(String),
}

impl std::fmt::Display for SettingsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let text = match self {
            Self::NotAFolder => "Choose an existing folder.",
            Self::AlreadyAdded => "That folder is already a skill directory.",
            Self::UnknownHarness => "That harness doesn't exist.",
            Self::UnknownDirectory => "That skill directory doesn't exist.",
            Self::EmptyName => "Give the harness a name.",
            Self::TemplateWithoutName => "The template must contain {name}.",
            Self::HarnessInUse => "Move or remove its skill directories first.",
            Self::InvalidTicketPattern => {
                "Choose a project-relative ticket pattern without parent components."
            }
            Self::Io(e) => return write!(f, "Could not save settings: {e}"),
        };
        f.write_str(text)
    }
}

impl std::error::Error for SettingsError {}

/// Lowercase ASCII letters, digits and dashes, e.g. `Team Skills` → `team-skills`.
pub(crate) fn slug(text: &str) -> String {
    let mut slug = String::new();
    for c in text.chars() {
        if c.is_ascii_alphanumeric() {
            slug.push(c.to_ascii_lowercase());
        } else if !slug.ends_with('-') && !slug.is_empty() {
            slug.push('-');
        }
    }
    let slug = slug.trim_end_matches('-').to_string();
    if slug.is_empty() {
        "folder".into()
    } else {
        slug
    }
}

/// `base`, or `base-2`, `base-3`… if `taken`.
pub(crate) fn unique_id(base: &str, taken: impl Fn(&str) -> bool) -> String {
    if !taken(base) {
        return base.to_string();
    }
    (2..)
        .map(|n| format!("{base}-{n}"))
        .find(|id| !taken(id))
        .expect("some id is free")
}
