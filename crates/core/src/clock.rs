use std::time::Instant;

/// Source of time, injectable so idle-stop and stuck-session rules can be tested
/// without waiting (see [`crate::testing::FakeClock`]).
pub trait Clock: Send + Sync {
    fn now(&self) -> Instant;
}

pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> Instant {
        Instant::now()
    }
}
