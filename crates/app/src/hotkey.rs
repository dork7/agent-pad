use std::path::PathBuf;
use std::sync::Mutex;
use std::time::Duration;

use serde::Serialize;
use agent_pad_core::Core;
use tauri::{AppHandle, State};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut};

use crate::config::{default_hotkey, load_config, parse_hotkey, save_config, Config};

pub struct HotkeySettings {
    path: PathBuf,
    config: Config,
    active: Option<Shortcut>,
    warning: Option<String>,
}

#[derive(Serialize)]
pub struct HotkeyInfo {
    hotkey: String,
    warning: Option<String>,
}

const DEFAULT_STUCK_THRESHOLD_SECONDS: u32 = 300;

impl HotkeySettings {
    pub fn register_at_startup(app: &AppHandle, path: PathBuf, core: &Core) -> Mutex<Self> {
        let (config, warning) = match load_config(&path) {
            Ok(config) => (config, None),
            Err(error) => (
                Config::default(),
                Some(format!("Could not read hotkey settings: {error}")),
            ),
        };
        let mut settings = Self {
            path,
            config,
            active: None,
            warning,
        };

        if let Some(seconds) = settings
            .config
            .stuck_threshold_seconds
            .filter(|seconds| *seconds > 0)
        {
            core.set_stuck_threshold(Duration::from_secs(u64::from(seconds)));
        }

        let default = default_hotkey(std::env::consts::OS);
        match register(app, &settings.config.hotkey) {
            Ok(shortcut) => settings.active = Some(shortcut),
            Err(error) => {
                settings.warn(error);
                if settings.config.hotkey != default {
                    match register(app, default) {
                        Ok(shortcut) => {
                            settings.active = Some(shortcut);
                            settings.config.hotkey = default.into();
                        }
                        Err(error) => settings.warn(error),
                    }
                }
            }
        }
        Mutex::new(settings)
    }

    fn stuck_threshold(&self) -> u32 {
        self.config
            .stuck_threshold_seconds
            .filter(|seconds| *seconds > 0)
            .unwrap_or(DEFAULT_STUCK_THRESHOLD_SECONDS)
    }

    fn warn(&mut self, message: String) {
        eprintln!("Agent Pad: {message}");
        self.warning = Some(match self.warning.take() {
            Some(previous) => format!("{previous}. {message}"),
            None => message,
        });
    }

    fn info(&self) -> HotkeyInfo {
        HotkeyInfo {
            hotkey: self.config.hotkey.clone(),
            warning: self.warning.clone(),
        }
    }

    fn rebind(&mut self, app: &AppHandle, input: &str) -> Result<HotkeyInfo, String> {
        let input = input.trim();
        let next = parse_hotkey(input)?;
        let mut config = self.config.clone();
        config.hotkey = input.into();

        let shortcuts = app.global_shortcut();
        let previous = self.active;
        let changed = previous != Some(next);
        if changed {
            // Claim the new shortcut while the old one is still available.
            register_shortcut(app, next, input)?;
            if let Some(previous) = previous {
                if let Err(error) = shortcuts.unregister(previous) {
                    let message = format!("Could not release the previous hotkey: {error}");
                    return Err(rollback_new_shortcut(app, next, message));
                }
            }
        }

        if let Err(error) = save_config(&self.path, &config) {
            let mut message = format!("Could not save the hotkey: {error}");
            if changed {
                if let Some(previous) = previous {
                    if let Err(error) = shortcuts.register(previous) {
                        self.active = None;
                        message
                            .push_str(&format!(". Could not restore the previous hotkey: {error}"));
                    }
                }
                message = rollback_new_shortcut(app, next, message);
            }
            return Err(message);
        }

        self.config = config;
        self.active = Some(next);
        self.warning = None;
        Ok(self.info())
    }
}

fn register(app: &AppHandle, input: &str) -> Result<Shortcut, String> {
    let shortcut = parse_hotkey(input)?;
    register_shortcut(app, shortcut, input)?;
    Ok(shortcut)
}

fn register_shortcut(app: &AppHandle, shortcut: Shortcut, input: &str) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    macos::check_available(shortcut).map_err(|error| format!("{input}: {error}"))?;

    app.global_shortcut()
        .register(shortcut)
        .map_err(|error| format!("Could not register {input}. It may already be in use: {error}"))
}

fn rollback_new_shortcut(app: &AppHandle, next: Shortcut, mut message: String) -> String {
    if let Err(error) = app.global_shortcut().unregister(next) {
        message.push_str(&format!(". Could not release the new hotkey: {error}"));
    }
    message
}

#[tauri::command]
pub fn get_hotkey(settings: State<'_, Mutex<HotkeySettings>>) -> HotkeyInfo {
    settings.lock().unwrap().info()
}

#[tauri::command]
pub fn set_hotkey(
    app: AppHandle,
    settings: State<'_, Mutex<HotkeySettings>>,
    hotkey: String,
) -> Result<HotkeyInfo, String> {
    settings.lock().unwrap().rebind(&app, &hotkey)
}

#[tauri::command]
pub fn stuck_threshold(settings: State<'_, Mutex<HotkeySettings>>) -> u32 {
    settings.lock().unwrap().stuck_threshold()
}

#[tauri::command]
pub fn set_stuck_threshold(
    settings: State<'_, Mutex<HotkeySettings>>,
    core: State<'_, Core>,
    seconds: u32,
) -> Result<(), String> {
    if seconds == 0 {
        return Err("Enter a positive number of seconds.".into());
    }
    let mut settings = settings.lock().unwrap();
    let mut config = settings.config.clone();
    config.stuck_threshold_seconds = Some(seconds);
    save_config(&settings.path, &config).map_err(|error| error.to_string())?;
    settings.config = config;
    core.set_stuck_threshold(Duration::from_secs(u64::from(seconds)));
    Ok(())
}

#[cfg(target_os = "macos")]
mod macos {
    use keycode::{KeyMap, KeyMappingCode};
    use tauri_plugin_global_shortcut::{Code, Modifiers, Shortcut};

    const EVENT_HOTKEY_EXISTS_ERR: i32 = -9878;

    unsafe extern "C" {
        fn agent_pad_probe_hotkey(key: u32, modifiers: u32) -> i32;
    }

    pub fn check_available(shortcut: Shortcut) -> Result<(), String> {
        let key = if shortcut.key == Code::PrintScreen {
            // Match the global shortcut plugin's Carbon mapping for this key.
            0x46
        } else {
            let Ok(code) = shortcut.key.to_string().parse::<KeyMappingCode>() else {
                return Ok(());
            };
            let key = KeyMap::from(code).mac;
            if key == u16::MAX {
                return Ok(());
            }
            u32::from(key)
        };
        let modifiers = [
            (Modifiers::SUPER, 1 << 8),
            (Modifiers::SHIFT, 1 << 9),
            (Modifiers::ALT, 1 << 11),
            (Modifiers::CONTROL, 1 << 12),
        ]
        .into_iter()
        .filter(|(flag, _)| shortcut.mods.contains(*flag))
        .map(|(_, carbon)| carbon)
        .sum();

        // The desktop setup and synchronous Tauri command run on the main thread.
        match unsafe { agent_pad_probe_hotkey(key, modifiers) } {
            0 => Ok(()),
            EVENT_HOTKEY_EXISTS_ERR => Err("This hotkey is already in use".into()),
            status => Err(format!("macOS could not check this hotkey ({status})")),
        }
    }
}
