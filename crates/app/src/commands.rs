use std::net::IpAddr;
use std::path::PathBuf;

use agent_pad_core::{
    ClaudeHooksPreview, CodexHooksPreview, ImplementError, Preset, Profile, Session, Snippet,
    Ticket, TicketSummary, CLAUDE_HOOKS_RESTART_NOTICE, CODEX_HOOKS_RESTART_NOTICE,
};
use agent_pad_core::{Core, Directory, Harness, Phone, PhoneError, Settings, SharingInfo, Skill};
use tauri::{AppHandle, Manager, State};

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

#[tauri::command]
pub fn preview_claude_hooks(core: State<'_, Core>) -> Result<ClaudeHooksPreview, String> {
    core.preview_claude_hooks()
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn apply_claude_hooks(
    core: State<'_, Core>,
    preview: ClaudeHooksPreview,
) -> Result<&'static str, String> {
    core.apply_claude_hooks(&preview)
        .map_err(|error| error.to_string())?;
    Ok(CLAUDE_HOOKS_RESTART_NOTICE)
}

#[tauri::command]
pub fn remove_claude_hooks(core: State<'_, Core>) -> Result<(), String> {
    core.remove_claude_hooks()
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn preview_codex_hooks(core: State<'_, Core>) -> Result<CodexHooksPreview, String> {
    core.preview_codex_hooks()
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn apply_codex_hooks(
    core: State<'_, Core>,
    preview: CodexHooksPreview,
) -> Result<&'static str, String> {
    core.apply_codex_hooks(&preview)
        .map_err(|error| error.to_string())?;
    Ok(CODEX_HOOKS_RESTART_NOTICE)
}

#[tauri::command]
pub fn remove_codex_hooks(core: State<'_, Core>) -> Result<(), String> {
    core.remove_codex_hooks().map_err(|error| error.to_string())
}

#[tauri::command]
pub fn list_sessions(core: State<'_, Core>) -> Vec<Session> {
    core.list_sessions()
}

// Core accepts a bare session ID for ticket operations, although session tracking is per harness.
fn require_unambiguous_session(core: &Core, session_id: &str) -> Result<(), String> {
    if core
        .list_sessions()
        .iter()
        .filter(|session| session.session_id == session_id)
        .take(2)
        .count()
        > 1
    {
        return Err("More than one harness is using this session ID. Open a new session to access its tickets.".into());
    }
    Ok(())
}

#[tauri::command]
pub fn pending_tickets(
    core: State<'_, Core>,
    session_id: String,
) -> Result<Vec<TicketSummary>, String> {
    require_unambiguous_session(&core, &session_id)?;
    core.pending_tickets(&session_id)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn ticket(
    core: State<'_, Core>,
    session_id: String,
    ticket_id: String,
) -> Result<Ticket, String> {
    require_unambiguous_session(&core, &session_id)?;
    core.ticket(&session_id, &ticket_id)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn ticket_pattern(core: State<'_, Core>) -> Option<String> {
    core.ticket_pattern()
}

#[tauri::command]
pub fn set_ticket_pattern(core: State<'_, Core>, pattern: Option<String>) -> Result<(), String> {
    core.set_ticket_pattern(pattern)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn implement_template(core: State<'_, Core>, harness: String) -> String {
    core.implement_template(&harness)
}

#[tauri::command]
pub fn set_implement_template(
    core: State<'_, Core>,
    harness: String,
    template: Option<String>,
) -> Result<(), String> {
    core.set_implement_template(&harness, template.as_deref())
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn implement_ticket(
    app: AppHandle,
    session_id: String,
    ticket_id: String,
) -> Result<(), String> {
    if !accessibility_granted() {
        return Err("Grant Accessibility access in the Pad to send tickets.".into());
    }
    // The paste worker hides the Pad on the main thread before typing.
    tauri::async_runtime::spawn_blocking(move || {
        let core = app.state::<Core>();
        require_unambiguous_session(&core, &session_id)?;
        core.implement_ticket(&session_id, &ticket_id)
            .map(|_| ())
            .map_err(|error| match error {
                ImplementError::Ticket(error) => error.to_string(),
                ImplementError::Blocked => "A pending ticket blocks this ticket.".into(),
                ImplementError::NotPending => "This ticket is no longer pending.".into(),
            })
    })
    .await
    .map_err(|error| error.to_string())?
}

#[tauri::command]
pub fn list_profiles(core: State<'_, Core>) -> (Vec<Profile>, Option<String>) {
    core.list_profiles()
}

#[tauri::command]
pub fn save_profile(core: State<'_, Core>, name: String) -> Result<(), String> {
    core.save_profile(&name).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn switch_profile(core: State<'_, Core>, name: String) -> Result<(), String> {
    core.switch_profile(&name)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn rename_profile(core: State<'_, Core>, name: String, new_name: String) -> Result<(), String> {
    core.rename_profile(&name, &new_name)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn delete_profile(core: State<'_, Core>, name: String) -> Result<(), String> {
    core.delete_profile(&name)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn list_presets(core: State<'_, Core>, skill_id: String) -> Vec<Preset> {
    core.list_presets(&skill_id)
}

#[tauri::command]
pub fn add_preset(core: State<'_, Core>, skill_id: String, text: String) -> Result<String, String> {
    core.add_preset(&skill_id, &text)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn update_preset(core: State<'_, Core>, id: String, text: String) -> Result<(), String> {
    core.update_preset(&id, &text)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn delete_preset(core: State<'_, Core>, id: String) -> Result<(), String> {
    core.delete_preset(&id).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn list_snippets(core: State<'_, Core>) -> Vec<Snippet> {
    core.list_snippets()
}

#[tauri::command]
pub fn add_snippet(core: State<'_, Core>, title: String, text: String) -> Result<String, String> {
    core.add_snippet(&title, &text)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn update_snippet(
    core: State<'_, Core>,
    id: String,
    title: String,
    text: String,
) -> Result<(), String> {
    core.update_snippet(&id, &title, &text)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn move_snippet(core: State<'_, Core>, id: String, index: usize) -> Result<(), String> {
    core.move_snippet(&id, index)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn delete_snippet(core: State<'_, Core>, id: String) -> Result<(), String> {
    core.delete_snippet(&id).map_err(|error| error.to_string())
}
