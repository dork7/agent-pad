use std::path::PathBuf;

use agent_pad_core::{hud_summaries, quota_ring, HudRing, Session, SessionStatus};

fn session(harness: &str, status: SessionStatus) -> Session {
    Session {
        session_id: format!("{harness}-{status:?}"),
        harness: harness.into(),
        project_folder: PathBuf::from("/work"),
        status,
        elapsed_seconds: 0,
        last_event_seconds: 0,
        tool_count: 0,
        agents: Vec::new(),
    }
}

#[test]
fn one_logo_per_live_harness_uses_its_most_urgent_status() {
    use SessionStatus::*;
    let summaries = hud_summaries(&[
        session("claude", Done),
        session("codex", Running),
        session("claude", Stalled),
        session("claude", Blocked),
        session("codex", Ended),
        session("other", Ended),
    ]);
    assert_eq!(summaries.len(), 2);
    assert_eq!(summaries[0].harness, "claude");
    assert_eq!(summaries[0].status, Blocked);
    assert_eq!(summaries[0].session_count, 3);
    assert_eq!(summaries[1].harness, "codex");
    assert_eq!(summaries[1].status, Running);
}

#[test]
fn quota_ring_uses_most_consumed_limit_and_neutral_for_missing_data() {
    assert_eq!(quota_ring(&[]), HudRing::Neutral);
    assert_eq!(quota_ring(&[0.69, 0.2]), HudRing::Green);
    assert_eq!(quota_ring(&[0.69, 0.70]), HudRing::Yellow);
    assert_eq!(quota_ring(&[0.89, 0.90]), HudRing::Red);
    assert_eq!(quota_ring(&[f64::NAN, -1.0, 2.0]), HudRing::Neutral);
}
