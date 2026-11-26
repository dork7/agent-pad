//! Headless core of Agent Pad.
//!
//! The Core owns all behavior. The desktop shell (Tauri) and the phone page only call into it:
//! the shell through [`Core`] methods, the phone and hook senders over HTTP.

pub mod clock;
pub mod harness;
mod hooks;
mod hud;
pub mod paste;
pub mod phones;
pub mod profiles;
pub mod saved_text;
mod server;
mod session_window;
mod sessions;
pub mod settings;
mod sharing;
pub mod skills;
pub mod testing;
mod tickets;
pub mod views;

use std::io;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, RwLock};

pub use clock::{Clock, SystemClock};
pub use harness::{CustomHarness, Harness};
pub use hooks::{ClaudeHooksPreview, CodexHooksPreview, CODEX_HOOKS_RESTART_NOTICE};
pub use hud::{hud_summaries, quota_ring, HudRing, HudSummary};
pub use paste::{PadWindow, Paster, SessionWindow, SystemPaster, SystemSessionWindow, WindowTarget};
pub use phones::{Phone, PhoneError};
pub use profiles::{Profile, ProfileError};
pub use saved_text::{Preset, PresetRef, Snippet, SNIPPETS_CHIP};
pub use sessions::{Session, SessionStatus, CLAUDE_HOOKS_RESTART_NOTICE};
pub use settings::{Settings, SettingsError};
pub use sharing::SharingInfo;
pub use skills::{Directory, Skill};
pub use tickets::{
    BlockedBy, ImplementError, Ticket, TicketError, TicketSummary, DEFAULT_IMPLEMENT_TEMPLATE,
    DEFAULT_TICKET_PATTERN,
};

/// Everything the Core needs from the outside world. Tests swap in a temp home,
/// a [`testing::RecordingPaster`] and a [`testing::FakeClock`].
pub struct CoreConfig {
    /// The user's home directory; skill locations are resolved under it.
    pub home: PathBuf,
    pub paster: Arc<dyn Paster>,
    pub clock: Arc<dyn Clock>,
}

/// A running Core with its local HTTP API on 127.0.0.1 and a random port.
/// Dropping it stops both the local API and any active sharing.
pub struct Core {
    state: Arc<State>,
    hook_listener: hooks::Listener,
    codex_hook_listener: hooks::Listener,
    server: server::Running,
    sharing: Mutex<sharing::Sharing>,
}

pub(crate) struct State {
    pub home: PathBuf,
    pub clock: Arc<dyn Clock>,
    pub pad: Arc<RwLock<Arc<dyn PadWindow>>>,
    pub window: Arc<RwLock<Arc<dyn SessionWindow>>>,
    pub pastes: paste::PasteQueue,
    pub settings: Mutex<Settings>,
    pub phones: Mutex<phones::Runtime>,
    pub sessions: Mutex<sessions::Sessions>,
}

/// Why a paste request was refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PasteError {
    /// No skill with this ID exists right now.
    UnknownSkill,
    /// The skill (or snippet) exists but is outside the requesting Phone's View.
    NotInView,
    /// The skill has no preset with this ID.
    UnknownPreset,
    /// No snippet has this ID.
    UnknownSnippet,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RunError {
    UnknownSession,
    UnknownSkill,
    WrongHarness,
    Blocked,
    WindowNotFound,
}

impl std::fmt::Display for RunError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::UnknownSession => "unknown session",
            Self::UnknownSkill => "unknown skill",
            Self::WrongHarness => "skill does not belong to this session's harness",
            Self::Blocked => "answer the session request first",
            Self::WindowNotFound => "session window was not found",
        })
    }
}

impl State {
    pub(crate) fn run_skill_in_session(&self, session_id: &str, skill_id: &str) -> Result<Skill, RunError> {
        let session = self.list_sessions().into_iter().find(|s| s.session_id == session_id)
            .ok_or(RunError::UnknownSession)?;
        if session.status == SessionStatus::Blocked { return Err(RunError::Blocked); }
        let skill = self.scan().into_iter().find(|s| s.id == skill_id)
            .ok_or(RunError::UnknownSkill)?;
        if skill.harness != session.harness { return Err(RunError::WrongHarness); }
        let target = self.sessions.lock().unwrap().target(session_id)
            .ok_or(RunError::WindowNotFound)?;
        if !self.pastes.paste_in_window(skill.invocation.clone(), true, target) {
            return Err(RunError::WindowNotFound);
        }
        Ok(skill)
    }

    /// Scans for skills now with the current settings. Skills are never cached.
    pub fn scan(&self) -> Vec<Skill> {
        let settings = self.settings.lock().unwrap().clone();
        skills::scan(&self.home, &settings)
    }

    pub fn harnesses(&self) -> Vec<Harness> {
        let settings = self.settings.lock().unwrap().clone();
        skills::harnesses(&skills::scan(&self.home, &settings), &settings)
    }

    /// Changes settings and saves them; nothing is saved if `change` fails.
    pub fn change_settings<T>(
        &self,
        change: impl FnOnce(&mut Settings) -> Result<T, SettingsError>,
    ) -> Result<T, SettingsError> {
        let mut settings = self.settings.lock().unwrap();
        let mut updated = settings.clone();
        let value = change(&mut updated)?;
        settings::save(&self.home, &updated).map_err(|e| SettingsError::Io(e.to_string()))?;
        *settings = updated;
        Ok(value)
    }

    /// Builds the Invocation for `skill_id` from a fresh scan and pastes it, waiting until the
    /// paste is done. The phone only ever sends an ID, so arbitrary text is never pasted.
    pub fn paste_skill(&self, skill_id: &str) -> Result<Skill, PasteError> {
        let skill = self
            .scan()
            .into_iter()
            .find(|s| s.id == skill_id)
            .ok_or(PasteError::UnknownSkill)?;
        self.paste_text(skill.invocation.clone());
        Ok(skill)
    }

    /// Pastes `text` through the queue, followed by Enter if that setting is on.
    pub(crate) fn paste_text(&self, text: String) {
        let enter = self.settings.lock().unwrap().send_enter;
        self.pastes.paste(text, enter);
    }
}

impl Core {
    pub fn run_skill_in_session(&self, session_id: &str, skill_id: &str) -> Result<Skill, RunError> {
        self.state.run_skill_in_session(session_id, skill_id)
    }

    pub fn start(config: CoreConfig) -> io::Result<Core> {
        let pad: Arc<RwLock<Arc<dyn PadWindow>>> =
            Arc::new(RwLock::new(Arc::new(paste::NoPadWindow)));
        let window: Arc<RwLock<Arc<dyn SessionWindow>>> =
            Arc::new(RwLock::new(Arc::new(SystemSessionWindow)));
        let loaded = settings::load(&config.home);
        let stuck_threshold = loaded.stuck_threshold_seconds;
        let settings = Mutex::new(loaded);
        let state = Arc::new(State {
            settings,
            phones: Default::default(),
            home: config.home,
            clock: config.clock,
            pastes: paste::PasteQueue::start(config.paster, pad.clone(), window.clone()),
            pad,
            window,
            sessions: Mutex::new(sessions::Sessions::with_threshold(stuck_threshold)),
        });
        let server = server::start(state.clone())?;
        let port = server.addr.port();
        let hook_listener = hooks::Listener::new(&state.home, port, hooks::Harness::Claude)?;
        let codex_hook_listener = hooks::Listener::new(&state.home, port, hooks::Harness::Codex)?;
        Ok(Core {
            state,
            server,
            sharing: Default::default(),
            hook_listener,
            codex_hook_listener,
        })
    }

    /// Address of the local HTTP API.
    pub fn addr(&self) -> SocketAddr {
        self.server.addr
    }

    /// Full URL for a path on the local HTTP API, e.g. `core.url("/skills")`.
    pub fn url(&self, path: &str) -> String {
        format!("http://{}{}", self.server.addr, path)
    }

    /// Scan for skills now. Skills are never cached: every call reads the disk.
    pub fn list_skills(&self) -> Vec<Skill> {
        self.state.scan()
    }

    /// Harness chips with at least one skill, rescanned on every call.
    pub fn list_harnesses(&self) -> Vec<Harness> {
        self.state.harnesses()
    }

    /// Registers the Pad window so the paste flow can hide it before pasting.
    pub fn set_pad_window(&self, pad: Arc<dyn PadWindow>) {
        *self.state.pad.write().unwrap() = pad;
    }

    /// Replace window raising, for example with a recording fake in tests.
    pub fn set_session_window(&self, window: Arc<dyn SessionWindow>) {
        *self.state.window.write().unwrap() = window;
    }

    /// Pastes the Invocation of `skill_id` at the cursor of the focused app, as a phone tap
    /// does. Returns the pasted skill.
    pub fn paste_skill(&self, skill_id: &str) -> Result<Skill, PasteError> {
        self.state.paste_skill(skill_id)
    }

    /// Current settings.
    pub fn settings(&self) -> Settings {
        self.state.settings.lock().unwrap().clone()
    }

    /// Turns "Send Enter after paste" on or off. Applies to the next paste and is saved.
    pub fn set_send_enter(&self, on: bool) -> io::Result<()> {
        let mut settings = self.state.settings.lock().unwrap();
        settings.send_enter = on;
        settings::save(&self.state.home, &settings)
    }

    /// Known Phones with name, online state, last seen and View.
    pub fn list_phones(&self) -> Vec<Phone> {
        self.state.list_phones()
    }

    /// Renames a Phone. Empty names are ignored.
    pub fn rename_phone(&self, id: &str, name: &str) -> Result<(), PhoneError> {
        let name = name.trim().to_string();
        self.state.update_phone(id, |p| {
            if !name.is_empty() {
                p.name = name;
            }
        })
    }

    /// Shuts a Phone out until it scans the QR code again. Its saved View is kept.
    pub fn disconnect_phone(&self, id: &str) -> Result<(), PhoneError> {
        self.state.disconnect_phone(id)
    }

    /// Deletes a Phone and its saved View, and shuts it out until it scans again.
    pub fn forget_phone(&self, id: &str) -> Result<(), PhoneError> {
        self.state.forget_phone(id)
    }

    /// Added Skill Directory chips with skill counts.
    pub fn list_directories(&self) -> Vec<Directory> {
        let settings = self.state.settings.lock().unwrap().clone();
        skills::directories(&skills::scan(&self.state.home, &settings), &settings)
    }

    /// Adds a folder as a Skill Directory of `harness`. Returns the new directory's ID.
    pub fn add_directory(&self, path: &Path, harness: &str) -> Result<String, SettingsError> {
        if !path.is_absolute() || !path.is_dir() {
            return Err(SettingsError::NotAFolder);
        }
        self.state.change_settings(|s| {
            if !harness::entries(s).iter().any(|h| h.id == harness) {
                return Err(SettingsError::UnknownHarness);
            }
            if s.directories.iter().any(|d| d.path == path) {
                return Err(SettingsError::AlreadyAdded);
            }
            let id = settings::unique_id(
                &settings::slug(&path.file_name().unwrap_or_default().to_string_lossy()),
                |id| s.directories.iter().any(|d| d.id == id),
            );
            s.directories.push(skills::AddedDirectory {
                id: id.clone(),
                path: path.to_path_buf(),
                harness: harness.to_string(),
            });
            Ok(id)
        })
    }

    /// Removes an added Skill Directory and drops its chip from every Phone's View.
    pub fn remove_directory(&self, id: &str) -> Result<(), SettingsError> {
        self.state.change_settings(|s| {
            let before = s.directories.len();
            s.directories.retain(|d| d.id != id);
            if s.directories.len() == before {
                return Err(SettingsError::UnknownDirectory);
            }
            let chip = format!("d:{id}");
            for phone in &mut s.phones {
                if let Some(view) = &mut phone.view {
                    view.retain(|c| *c != chip);
                }
            }
            Ok(())
        })
    }

    /// Defines a custom Harness. Returns its ID.
    pub fn add_custom_harness(&self, name: &str, template: &str) -> Result<String, SettingsError> {
        let name = name.trim();
        if name.is_empty() {
            return Err(SettingsError::EmptyName);
        }
        if !harness::valid_template(template) {
            return Err(SettingsError::TemplateWithoutName);
        }
        self.state.change_settings(|s| {
            let id = settings::unique_id(&format!("custom-{}", settings::slug(name)), |id| {
                harness::entries(s).iter().any(|h| h.id == id)
            });
            s.custom_harnesses.push(CustomHarness {
                id: id.clone(),
                name: name.to_string(),
                template: template.to_string(),
            });
            Ok(id)
        })
    }

    /// Deletes a custom Harness. Refused while Skill Directories are assigned to it.
    pub fn remove_custom_harness(&self, id: &str) -> Result<(), SettingsError> {
        self.state.change_settings(|s| {
            if !s.custom_harnesses.iter().any(|h| h.id == id) {
                return Err(SettingsError::UnknownHarness);
            }
            if s.directories.iter().any(|d| d.harness == id) {
                return Err(SettingsError::HarnessInUse);
            }
            s.custom_harnesses.retain(|h| h.id != id);
            let chip = format!("h:{id}");
            for phone in &mut s.phones {
                if let Some(view) = &mut phone.view {
                    view.retain(|c| *c != chip);
                }
            }
            Ok(())
        })
    }

    /// Sets a Harness's Invocation template. `{name}` is replaced by the skill name and one
    /// trailing space is always added.
    pub fn set_template(&self, harness: &str, template: &str) -> Result<(), SettingsError> {
        if !harness::valid_template(template) {
            return Err(SettingsError::TemplateWithoutName);
        }
        self.state.change_settings(|s| {
            if let Some(custom) = s.custom_harnesses.iter_mut().find(|h| h.id == harness) {
                custom.template = template.to_string();
            } else if harness::def(harness).is_some() {
                s.templates
                    .insert(harness.to_string(), template.to_string());
            } else {
                return Err(SettingsError::UnknownHarness);
            }
            Ok(())
        })
    }

    /// Restores a registry Harness's documented template.
    pub fn reset_template(&self, harness: &str) -> Result<(), SettingsError> {
        self.state.change_settings(|s| {
            if harness::def(harness).is_none() {
                return Err(SettingsError::UnknownHarness);
            }
            s.templates.remove(harness);
            Ok(())
        })
    }

    /// Every Harness with its current template, including ones with no skills yet.
    pub fn list_templates(&self) -> Vec<(String, String, String)> {
        let settings = self.state.settings.lock().unwrap().clone();
        harness::entries(&settings)
            .into_iter()
            .map(|e| (e.id, e.name, e.template))
            .collect()
    }

    /// Every chip that exists now (`h:<harness>` for harnesses with skills, `d:<directory>`
    /// for added directories), for the Pad's palette.
    pub fn list_chips(&self) -> Vec<String> {
        self.state.chips()
    }

    /// Replaces a Phone's View: `None` for everything, or a list of chips. Applies live.
    pub fn set_phone_view(&self, id: &str, view: Option<Vec<String>>) -> Result<(), PhoneError> {
        self.state.set_view(id, view)
    }

    /// Adds a chip to a Phone's View (dropping it on the Phone's card).
    pub fn add_chip_to_phone(&self, id: &str, chip: &str) -> Result<(), PhoneError> {
        self.state.add_chip(id, chip)
    }

    /// Removes a chip from a Phone's View (its ×).
    pub fn remove_chip_from_phone(&self, id: &str, chip: &str) -> Result<(), PhoneError> {
        self.state.remove_chip(id, chip)
    }

    /// Saved Profiles and the active one.
    pub fn list_profiles(&self) -> (Vec<Profile>, Option<String>) {
        let settings = self.state.settings.lock().unwrap();
        (settings.profiles.clone(), settings.active_profile.clone())
    }

    /// Saves every known Phone's View under `name`, replacing a Profile with that name.
    pub fn save_profile(&self, name: &str) -> Result<(), ProfileError> {
        self.state.save_profile(name)
    }

    /// Applies a Profile's Views to the Phones it knows, live.
    pub fn switch_profile(&self, name: &str) -> Result<(), ProfileError> {
        self.state.switch_profile(name)
    }

    pub fn rename_profile(&self, name: &str, new_name: &str) -> Result<(), ProfileError> {
        self.state.rename_profile(name, new_name)
    }

    pub fn delete_profile(&self, name: &str) -> Result<(), ProfileError> {
        self.state.delete_profile(name)
    }
}
