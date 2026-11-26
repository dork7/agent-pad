//! Ticket 23: Home orders visible sessions by urgency.

use std::sync::Arc;
use std::time::Duration;

use agent_pad_core::testing::{FakeClock, RecordingPaster};
use agent_pad_core::{Core, CoreConfig, Session};
use serde_json::json;

fn hook(core: &Core, id: &str, event: &str) {
    ureq::post(&core.url("/hooks/claude"))
        .send_json(json!({
            "session_id": id,
            "cwd": format!("/work/{id}"),
            "hook_event_name": event,
            "source": "startup"
        }))
        .unwrap();
}

#[test]
fn home_orders_sessions_by_urgency_and_reports_the_last_event() {
    let home = tempfile::TempDir::new().unwrap();
    let clock = Arc::new(FakeClock::new());
    let core = Core::start(CoreConfig {
        home: home.path().into(),
        paster: Arc::new(RecordingPaster::new()),
        clock: clock.clone(),
    })
    .unwrap();
    for id in ["done", "running", "stalled", "blocked"] {
        hook(&core, id, "SessionStart");
    }
    hook(&core, "stalled", "UserPromptSubmit");
    clock.advance(Duration::from_secs(301));
    hook(&core, "running", "UserPromptSubmit");
    hook(&core, "blocked", "PermissionRequest");

    let listed: Vec<Session> = ureq::get(&core.url("/sessions"))
        .call()
        .unwrap()
        .into_json()
        .unwrap();
    assert_eq!(
        listed
            .iter()
            .map(|s| s.session_id.as_str())
            .collect::<Vec<_>>(),
        ["blocked", "stalled", "running", "done"]
    );
    assert_eq!(listed[0].last_event_seconds, 0);
    assert_eq!(listed[1].last_event_seconds, 301);
}
