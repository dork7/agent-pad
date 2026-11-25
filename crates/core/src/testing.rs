//! Test fakes for the Core's edges: a recording Paster and a controllable clock.

use std::sync::Mutex;
use std::time::{Duration, Instant};

use crate::{Clock, Paster};

/// Records every paste and key press instead of touching the clipboard or keyboard.
#[derive(Default)]
pub struct RecordingPaster {
    events: Mutex<Vec<Recorded>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Recorded {
    Paste(String),
    Enter,
}

impl RecordingPaster {
    pub fn new() -> Self {
        Self::default()
    }

    /// Every text pasted so far, in order (key presses left out).
    pub fn pasted(&self) -> Vec<String> {
        self.events
            .lock()
            .unwrap()
            .iter()
            .filter_map(|e| match e {
                Recorded::Paste(text) => Some(text.clone()),
                Recorded::Enter => None,
            })
            .collect()
    }

    /// Every paste and Enter press so far, in order.
    pub fn events(&self) -> Vec<Recorded> {
        self.events.lock().unwrap().clone()
    }
}

impl Paster for RecordingPaster {
    fn paste(&self, text: &str) {
        self.events
            .lock()
            .unwrap()
            .push(Recorded::Paste(text.to_string()));
    }

    fn press_enter(&self) {
        self.events.lock().unwrap().push(Recorded::Enter);
    }
}

/// A clock that only moves when told to.
pub struct FakeClock {
    start: Instant,
    elapsed: Mutex<Duration>,
}

impl FakeClock {
    pub fn new() -> Self {
        Self {
            start: Instant::now(),
            elapsed: Mutex::new(Duration::ZERO),
        }
    }

    pub fn advance(&self, by: Duration) {
        *self.elapsed.lock().unwrap() += by;
    }
}

impl Default for FakeClock {
    fn default() -> Self {
        Self::new()
    }
}

impl Clock for FakeClock {
    fn now(&self) -> Instant {
        self.start + *self.elapsed.lock().unwrap()
    }
}
