//! Headless core of Skills Pad.
//!
//! The Core owns all behavior. The desktop shell (Tauri) and the phone page only call into it:
//! the shell through [`Core`] methods, the phone and hook senders over HTTP.

pub mod clock;
pub mod paste;
mod server;
pub mod skills;
pub mod testing;

use std::io;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;

pub use clock::{Clock, SystemClock};
pub use paste::Paster;
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
/// Dropping it stops the server.
pub struct Core {
    state: Arc<State>,
    server: server::Running,
}

pub(crate) struct State {
    pub home: PathBuf,
    #[allow(dead_code)] // used from ticket 04 (paste) on
    pub paster: Arc<dyn Paster>,
    #[allow(dead_code)] // used from ticket 03 (idle stop) on
    pub clock: Arc<dyn Clock>,
}

impl Core {
    pub fn start(config: CoreConfig) -> io::Result<Core> {
        let state = Arc::new(State {
            home: config.home,
            paster: config.paster,
            clock: config.clock,
        });
        let server = server::start(state.clone())?;
        Ok(Core { state, server })
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
}
