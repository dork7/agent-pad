//! Headless core of Skills Pad.
//!
//! The Core owns all behavior. The desktop shell (Tauri) and the phone page only call into it:
//! the shell through [`Core`] methods, the phone and hook senders over HTTP.

pub mod clock;
pub mod harness;
pub mod paste;
mod server;
pub mod settings;
mod sharing;
pub mod skills;
pub mod testing;

use std::io;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, RwLock};

pub use clock::{Clock, SystemClock};
pub use harness::Harness;
pub use paste::{PadWindow, Paster, SystemPaster};
pub use settings::Settings;
pub use sharing::SharingInfo;
pub use skills::Skill;

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
    server: server::Running,
    sharing: std::sync::Mutex<sharing::Sharing>,
}

pub(crate) struct State {
    pub home: PathBuf,
    pub clock: Arc<dyn Clock>,
    pub pad: Arc<RwLock<Arc<dyn PadWindow>>>,
    pub pastes: paste::PasteQueue,
    pub settings: Mutex<Settings>,
}

/// Why a paste request was refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PasteError {
    /// No skill with this ID exists right now.
    UnknownSkill,
}

impl State {
    /// Builds the Invocation for `skill_id` from a fresh scan and pastes it, waiting until the
    /// paste is done. The phone only ever sends an ID, so arbitrary text is never pasted.
    pub fn paste_skill(&self, skill_id: &str) -> Result<Skill, PasteError> {
        let skill = skills::scan(&self.home)
            .into_iter()
            .find(|s| s.id == skill_id)
            .ok_or(PasteError::UnknownSkill)?;
        let enter = self.settings.lock().unwrap().send_enter;
        self.pastes.paste(skill.invocation.clone(), enter);
        Ok(skill)
    }
}

impl Core {
    pub fn start(config: CoreConfig) -> io::Result<Core> {
        let pad: Arc<RwLock<Arc<dyn PadWindow>>> =
            Arc::new(RwLock::new(Arc::new(paste::NoPadWindow)));
        let settings = Mutex::new(settings::load(&config.home));
        let state = Arc::new(State {
            settings,
            home: config.home,
            clock: config.clock,
            pastes: paste::PasteQueue::start(config.paster, pad.clone()),
            pad,
        });
        let server = server::start(state.clone())?;
        Ok(Core {
            state,
            server,
            sharing: Default::default(),
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
        skills::scan(&self.state.home)
    }

    /// Harness chips with at least one skill, rescanned on every call.
    pub fn list_harnesses(&self) -> Vec<Harness> {
        skills::harnesses(&skills::scan(&self.state.home))
    }

    /// Registers the Pad window so the paste flow can hide it before pasting.
    pub fn set_pad_window(&self, pad: Arc<dyn PadWindow>) {
        *self.state.pad.write().unwrap() = pad;
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
}
