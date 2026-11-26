use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::{Session, SessionStatus};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HudRing {
    Neutral,
    Green,
    Yellow,
    Red,
}

/// Fractions used by the most consumed quota. Invalid and absent readings carry no colour.
pub fn quota_ring(used_fractions: &[f64]) -> HudRing {
    let maximum = used_fractions
        .iter()
        .copied()
        .filter(|value| value.is_finite() && (0.0..=1.0).contains(value))
        .fold(None::<f64>, |previous, value| Some(previous.map_or(value, |p| p.max(value))));
    match maximum {
        None => HudRing::Neutral,
        Some(value) if value >= 0.9 => HudRing::Red,
        Some(value) if value >= 0.7 => HudRing::Yellow,
        Some(_) => HudRing::Green,
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HudSummary {
    pub harness: String,
    pub status: SessionStatus,
    pub session_count: usize,
    pub ring: HudRing,
}

fn urgency(status: SessionStatus) -> u8 {
    match status {
        SessionStatus::Blocked => 0,
        SessionStatus::Stalled => 1,
        SessionStatus::Running => 2,
        SessionStatus::Done => 3,
        SessionStatus::Ended => 4,
    }
}

/// A live harness appears once, even when several sessions report at once.
pub fn hud_summaries(sessions: &[Session]) -> Vec<HudSummary> {
    let mut by_harness = BTreeMap::<String, HudSummary>::new();
    for session in sessions.iter().filter(|session| session.status != SessionStatus::Ended) {
        let summary = by_harness.entry(session.harness.clone()).or_insert_with(|| HudSummary {
            harness: session.harness.clone(),
            status: session.status,
            session_count: 0,
            ring: HudRing::Neutral,
        });
        summary.session_count += 1;
        if urgency(session.status) < urgency(summary.status) {
            summary.status = session.status;
        }
    }
    by_harness.into_values().collect()
}
