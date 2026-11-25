#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

mod commands;
mod config;
mod hotkey;

use std::sync::Arc;

use skills_pad_core::{Core, CoreConfig, PadWindow, SystemClock, SystemPaster};
use tauri::image::Image;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Emitter, Manager, WebviewWindow, WindowEvent};
use tauri_plugin_global_shortcut::ShortcutState;

const PAD: &str = "pad";
const SHARING_CHANGED: &str = "sharing-changed";

/// Lets the Core's paste flow move the Pad out of the way so the paste lands in the app the
/// user was working in.
struct TauriPad(AppHandle);

impl PadWindow for TauriPad {
    fn hide_if_visible(&self) -> bool {
        let Some(pad) = self.0.get_webview_window(PAD) else {
            return false;
        };
        if !pad.is_visible().unwrap_or(false) {
            return false;
        }
        // On macOS, hiding the whole app (not just the window) hands focus back to the
        // previously active app.
        #[cfg(target_os = "macos")]
        let hidden = self.0.hide();
        #[cfg(not(target_os = "macos"))]
        let hidden = pad.hide();
        if let Err(error) = hidden {
            eprintln!("Skills Pad could not hide before pasting: {error}");
        }
        true
    }
}

fn show_pad(pad: &WebviewWindow) -> tauri::Result<()> {
    #[cfg(target_os = "macos")]
    pad.app_handle().show()?;
    pad.show()?;
    pad.unminimize()?;
    pad.set_focus()
}

fn toggle_pad(pad: &WebviewWindow) -> tauri::Result<()> {
    if pad.is_visible()? && !pad.is_minimized()? {
        pad.hide()
    } else {
        show_pad(pad)
    }
}

fn tray_menu_action(app: &AppHandle, action: &str) -> tauri::Result<()> {
    let Some(pad) = app.get_webview_window(PAD) else {
        return Ok(());
    };

    match action {
        "open-pad" => show_pad(&pad)?,
        "open-on-phone" => {
            show_pad(&pad)?;
            let core = app.state::<Core>();
            let error = if core.sharing_status().is_none() {
                core.start_sharing().err().map(|error| error.to_string())
            } else {
                None
            };
            app.emit(SHARING_CHANGED, error)?;
        }
        "stop-sharing" => {
            app.state::<Core>().stop_sharing();
            app.emit(SHARING_CHANGED, None::<String>)?;
        }
        "quit" => app.exit(0),
        _ => {}
    }
    Ok(())
}

fn build_tray(app: &AppHandle) -> tauri::Result<()> {
    let open = MenuItem::with_id(app, "open-pad", "Open Pad", true, None::<&str>)?;
    let phone = MenuItem::with_id(app, "open-on-phone", "Open on phone", true, None::<&str>)?;
    let stop = MenuItem::with_id(app, "stop-sharing", "Stop sharing", true, None::<&str>)?;
    let separator = PredefinedMenuItem::separator(app)?;
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&open, &phone, &stop, &separator, &quit])?;

    let icon = if cfg!(target_os = "macos") {
        include_bytes!("../icons/tray.png").as_slice()
    } else {
        include_bytes!("../icons/32x32.png").as_slice()
    };
    TrayIconBuilder::new()
        .icon(Image::from_bytes(icon)?)
        .icon_as_template(cfg!(target_os = "macos"))
        .tooltip("Skills Pad")
        .menu(&menu)
        .on_menu_event(|app, event| {
            if let Err(error) = tray_menu_action(app, event.id().as_ref()) {
                eprintln!("Skills Pad tray action failed: {error}");
            }
        })
        .build(app)?;
    Ok(())
}

fn main() -> tauri::Result<()> {
    tauri::Builder::default()
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, _, event| {
                    if event.state == ShortcutState::Pressed {
                        if let Some(pad) = app.get_webview_window(PAD) {
                            if let Err(error) = toggle_pad(&pad) {
                                eprintln!("Skills Pad hotkey action failed: {error}");
                            }
                        }
                    }
                })
                .build(),
        )
        .setup(|app| {
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            let core = Core::start(CoreConfig {
                home: app.path().home_dir()?,
                paster: Arc::new(SystemPaster),
                clock: Arc::new(SystemClock),
            })?;
            core.set_pad_window(Arc::new(TauriPad(app.handle().clone())));
            app.manage(core);

            let path = app.path().app_config_dir()?.join("config.json");
            app.manage(hotkey::HotkeySettings::register_at_startup(
                app.handle(),
                path,
            ));
            build_tray(app.handle())?;
            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                if window.label() == PAD {
                    api.prevent_close();
                    if let Err(error) = window.hide() {
                        eprintln!("Skills Pad could not hide the window: {error}");
                    }
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::list_skills,
            commands::list_harnesses,
            commands::start_sharing,
            commands::stop_sharing,
            commands::sharing_status,
            commands::set_sharing_address_override,
            commands::sharing_address,
            commands::settings,
            commands::set_send_enter,
            commands::list_phones,
            commands::rename_phone,
            commands::disconnect_phone,
            commands::forget_phone,
            hotkey::get_hotkey,
            hotkey::set_hotkey,
        ])
        .run(tauri::generate_context!())
}
