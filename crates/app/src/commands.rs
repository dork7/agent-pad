use std::net::IpAddr;
use std::path::PathBuf;

use skills_pad_core::{Core, Directory, Harness, Phone, PhoneError, Settings, SharingInfo, Skill};
use tauri::State;

use crate::accessibility::{accessibility_granted, require_access};

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
    require_access(accessibility_granted())?;
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
pub fn set_sharing_address_override(
    core: State<'_, Core>,
    address: Option<String>,
) -> Result<(), String> {
    let address = address
        .map(|address| address.trim().parse::<IpAddr>())
        .transpose()
        .map_err(|error| error.to_string())?;
    if address.is_some_and(|address| address.is_unspecified()) {
        return Err("Sharing requires a specific network address.".into());
    }
    core.set_sharing_address_override(address);
    // Core keeps the previous value when saving fails.
    if core.sharing_address_override() != address {
        return Err("Could not save the network address override.".into());
    }
    Ok(())
}

#[tauri::command]
pub fn sharing_address_override(core: State<'_, Core>) -> Option<IpAddr> {
    core.sharing_address_override()
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
        PhoneError::UnknownChip => "That harness or directory no longer exists.".into(),
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

#[tauri::command]
pub fn list_chips(core: State<'_, Core>) -> Vec<String> {
    core.list_chips()
}

#[tauri::command]
pub fn set_phone_view(
    core: State<'_, Core>,
    id: String,
    view: Option<Vec<String>>,
) -> Result<(), String> {
    core.set_phone_view(&id, view).map_err(phone_error)
}

#[tauri::command]
pub fn add_chip_to_phone(core: State<'_, Core>, id: String, chip: String) -> Result<(), String> {
    core.add_chip_to_phone(&id, &chip).map_err(phone_error)
}

#[tauri::command]
pub fn remove_chip_from_phone(
    core: State<'_, Core>,
    id: String,
    chip: String,
) -> Result<(), String> {
    core.remove_chip_from_phone(&id, &chip).map_err(phone_error)
}

#[tauri::command]
pub fn list_directories(core: State<'_, Core>) -> Vec<Directory> {
    core.list_directories()
}

#[tauri::command]
pub fn add_directory(
    core: State<'_, Core>,
    path: PathBuf,
    harness: String,
) -> Result<String, String> {
    core.add_directory(&path, &harness)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn remove_directory(core: State<'_, Core>, id: String) -> Result<(), String> {
    core.remove_directory(&id)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn add_custom_harness(
    core: State<'_, Core>,
    name: String,
    template: String,
) -> Result<String, String> {
    core.add_custom_harness(&name, &template)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn remove_custom_harness(core: State<'_, Core>, id: String) -> Result<(), String> {
    core.remove_custom_harness(&id)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn list_templates(core: State<'_, Core>) -> Vec<(String, String, String)> {
    core.list_templates()
}

#[tauri::command]
pub fn set_template(
    core: State<'_, Core>,
    harness: String,
    template: String,
) -> Result<(), String> {
    core.set_template(&harness, &template)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn reset_template(core: State<'_, Core>, harness: String) -> Result<(), String> {
    core.reset_template(&harness)
        .map_err(|error| error.to_string())
}
