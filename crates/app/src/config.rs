use std::fs::{self, File};
use std::io::{self, BufReader, Write};
use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use tauri_plugin_global_shortcut::Shortcut;

pub fn default_hotkey(os: &str) -> &'static str {
    match os {
        "macos" => "Cmd+Option+S",
        _ => "Ctrl+Alt+S",
    }
}

pub fn parse_hotkey(input: &str) -> Result<Shortcut, String> {
    input
        .trim()
        .parse::<Shortcut>()
        .map_err(|error| format!("Invalid hotkey: {error}"))
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct Config {
    pub hotkey: String,
    #[serde(flatten)]
    extra: Map<String, Value>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            hotkey: default_hotkey(std::env::consts::OS).into(),
            extra: Map::new(),
        }
    }
}

pub fn load_config(path: &Path) -> io::Result<Config> {
    match File::open(path) {
        Ok(file) => Ok(serde_json::from_reader(BufReader::new(file))?),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(Config::default()),
        Err(error) => Err(error),
    }
}

pub fn save_config(path: &Path, config: &Config) -> io::Result<()> {
    let directory = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(directory)?;
    let mut file = tempfile::NamedTempFile::new_in(directory)?;
    serde_json::to_writer_pretty(&mut file, config)?;
    file.write_all(b"\n")?;
    file.as_file().sync_all()?;
    file.persist(path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use tauri_plugin_global_shortcut::{Code, Modifiers};

    #[test]
    fn platform_defaults_have_the_expected_modifiers() {
        let mac = parse_hotkey(default_hotkey("macos")).unwrap();
        let windows = parse_hotkey(default_hotkey("windows")).unwrap();
        assert!(mac.matches(Modifiers::SUPER | Modifiers::ALT, Code::KeyS));
        assert!(windows.matches(Modifiers::CONTROL | Modifiers::ALT, Code::KeyS));
    }

    #[test]
    fn parses_aliases_and_rejects_malformed_combinations() {
        assert_eq!(
            parse_hotkey("  Command + Alt + S ").unwrap(),
            parse_hotkey("Cmd+Option+S").unwrap()
        );
        for invalid in ["", "Ctrl+Alt", "Ctrl++S", "Ctrl+S+A", "Ctrl+Unknown"] {
            assert!(parse_hotkey(invalid).is_err(), "{invalid}");
        }
    }

    #[test]
    fn missing_config_uses_the_platform_default() {
        let directory = tempfile::tempdir().unwrap();
        let config = load_config(&directory.path().join("config.json")).unwrap();
        assert_eq!(config.hotkey, default_hotkey(std::env::consts::OS));
    }

    #[test]
    fn config_round_trips_and_preserves_other_fields() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("settings/config.json");
        let mut config = Config {
            hotkey: "Alt+Shift+K".into(),
            ..Config::default()
        };
        save_config(&path, &config).unwrap();
        assert_eq!(load_config(&path).unwrap().hotkey, "Alt+Shift+K");

        fs::write(&path, r#"{"future_setting":{"enabled":true}}"#).unwrap();
        config = load_config(&path).unwrap();
        assert_eq!(config.hotkey, default_hotkey(std::env::consts::OS));
        config.hotkey = "Ctrl+Shift+K".into();
        save_config(&path, &config).unwrap();

        let saved: Value = serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
        assert_eq!(
            saved,
            json!({"hotkey":"Ctrl+Shift+K","future_setting":{"enabled":true}})
        );
    }

    #[test]
    fn malformed_config_returns_an_error() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("config.json");
        fs::write(&path, "invalid json").unwrap();
        assert!(load_config(&path).is_err());
    }
}
