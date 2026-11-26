use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use agent_pad_core::{hud_summaries, Core, HudSummary};
use tauri::{AppHandle, Manager, PhysicalPosition, WebviewUrl, WebviewWindowBuilder};

use crate::config::{load_config, save_config};

pub const HUD: &str = "hud";

pub struct HudState {
    path: PathBuf,
    move_generation: Arc<Mutex<u64>>,
}

impl HudState {
    pub fn new(path: PathBuf) -> Self {
        Self { path, move_generation: Arc::new(Mutex::new(0)) }
    }
}

pub fn start(app: &AppHandle) -> tauri::Result<()> {
    let state = app.state::<HudState>();
    let config = load_config(&state.path).unwrap_or_default();
    let builder = WebviewWindowBuilder::new(app, HUD, WebviewUrl::App("hud.html".into()))
        .title("Agent Pad HUD")
        .inner_size(66.0, 112.0)
        .resizable(false)
        .decorations(false)
        .transparent(true)
        .always_on_top(true)
        .skip_taskbar(true)
        .visible(config.hud_visible);
    let window = builder.build()?;
    if let Some((x, y)) = config.hud_position {
        window.set_position(PhysicalPosition::new(x, y))?;
    }
    dock(app);
    Ok(())
}

#[tauri::command]
pub fn hud_summaries_command(core: tauri::State<'_, Core>) -> Vec<HudSummary> {
    hud_summaries(&core.list_sessions())
}

#[tauri::command]
pub fn hud_visible(state: tauri::State<'_, HudState>) -> bool {
    load_config(&state.path).map(|config| config.hud_visible).unwrap_or(true)
}

#[tauri::command]
pub fn set_hud_visible(app: AppHandle, visible: bool) -> Result<(), String> {
    let state = app.state::<HudState>();
    let mut config = load_config(&state.path).map_err(|error| error.to_string())?;
    config.hud_visible = visible;
    save_config(&state.path, &config).map_err(|error| error.to_string())?;
    let hud = app.get_webview_window(HUD).ok_or("HUD window is unavailable")?;
    if visible { hud.show() } else { hud.hide() }.map_err(|error| error.to_string())
}

pub fn moved(app: &AppHandle) {
    let state = app.state::<HudState>();
    let generation = {
        let mut value = state.move_generation.lock().unwrap();
        *value += 1;
        *value
    };
    let generations = state.move_generation.clone();
    let app = app.clone();
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(350));
        if *generations.lock().unwrap() != generation { return; }
        let app_for_callback = app.clone();
        let _ = app.run_on_main_thread(move || dock(&app_for_callback));
    });
}

fn dock(app: &AppHandle) {
    let Some(window) = app.get_webview_window(HUD) else { return };
    let (Ok(position), Ok(size), Ok(Some(monitor))) =
        (window.outer_position(), window.outer_size(), window.current_monitor()) else { return };
    let area = monitor.position();
    let extent = monitor.size();
    let left = area.x;
    let right = area.x + extent.width as i32 - size.width as i32;
    let top = area.y;
    let bottom = area.y + extent.height as i32 - size.height as i32;
    let x = position.x.clamp(left, right.max(left));
    let y = position.y.clamp(top, bottom.max(top));
    let distances = [(x - left, left, y), (right - x, right, y), (y - top, x, top), (bottom - y, x, bottom)];
    let (_, snapped_x, snapped_y) = *distances.iter().min_by_key(|item| item.0).unwrap();
    let snapped = PhysicalPosition::new(snapped_x, snapped_y);
    if position != snapped {
        let _ = window.set_position(snapped);
    }
    let state = app.state::<HudState>();
    if let Ok(mut config) = load_config(&state.path) {
        config.hud_position = Some((snapped_x, snapped_y));
        if let Err(error) = save_config(&state.path, &config) {
            eprintln!("Agent Pad could not save HUD position: {error}");
        }
    }
}
