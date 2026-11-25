use std::net::IpAddr;

use skills_pad_core::{Core, Harness, Phone, PhoneError, Settings, SharingInfo, Skill};
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

#[tauri::command]
pub fn settings(core: State<'_, Core>) -> Settings {
    core.settings()
}

#[tauri::command]
pub fn set_send_enter(core: State<'_, Core>, on: bool) -> Result<(), String> {
    core.set_send_enter(on).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn list_phones(core: State<'_, Core>) -> Vec<Phone> {
    core.list_phones()
}

fn phone_error(error: PhoneError) -> String {
    match error {
        PhoneError::UnknownPhone => "That phone is no longer known.".into(),
    }
}

#[tauri::command]
pub fn rename_phone(core: State<'_, Core>, id: String, name: String) -> Result<(), String> {
    core.rename_phone(&id, &name).map_err(phone_error)
}

#[tauri::command]
pub fn disconnect_phone(core: State<'_, Core>, id: String) -> Result<(), String> {
    core.disconnect_phone(&id).map_err(phone_error)
}

#[tauri::command]
pub fn forget_phone(core: State<'_, Core>, id: String) -> Result<(), String> {
    core.forget_phone(&id).map_err(phone_error)
}
