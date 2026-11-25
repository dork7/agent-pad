//! Test fakes for the Core's edges: a recording Paster and a controllable clock.

use std::sync::Mutex;
use std::time::{Duration, Instant};

use crate::{Clock, Paster};

/// Records every paste instead of touching the clipboard or keyboard.
#[derive(Default)]
pub struct RecordingPaster {
    pasted: Mutex<Vec<String>>,
}

impl RecordingPaster {
    pub fn new() -> Self {
        Self::default()
    }

    /// Every text pasted so far, in order.
    pub fn pasted(&self) -> Vec<String> {
        self.pasted.lock().unwrap().clone()
    }
}

impl Paster for RecordingPaster {
    fn paste(&self, text: &str) {
        self.pasted.lock().unwrap().push(text.to_string());
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
