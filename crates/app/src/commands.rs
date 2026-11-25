use std::net::IpAddr;

use skills_pad_core::{Core, Harness, SharingInfo, Skill};
use tauri::State;

#[tauri::command]
pub fn list_skills(core: State<'_, Core>) -> Vec<Skill> {
    core.list_skills()
}

#[tauri::command]
pub fn list_harnesses(core: State<'_, Core>) -> Vec<Harness> {
    core.list_harnesses()
}

#[tauri::command]
pub fn start_sharing(core: State<'_, Core>) -> Result<SharingInfo, String> {
    core.start_sharing().map_err(|error| error.to_string())
}

#[tauri::command]
pub fn stop_sharing(core: State<'_, Core>) {
    core.stop_sharing();
}

#[tauri::command]
pub fn sharing_status(core: State<'_, Core>) -> Option<SharingInfo> {
    core.sharing_status()
}

#[tauri::command]
pub fn set_sharing_address_override(core: State<'_, Core>, address: Option<IpAddr>) {
    core.set_sharing_address_override(address);
}

#[tauri::command]
pub fn sharing_address(core: State<'_, Core>) -> Result<IpAddr, String> {
    core.sharing_address().map_err(|error| error.to_string())
}
