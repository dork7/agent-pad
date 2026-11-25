//! Ticket 20: "Implement" pastes an implement command for a pending ticket, then Enter.

use std::fs;
use std::net::{Ipv4Addr, SocketAddr};
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};
use serde_json::{json, Value};
use skills_pad_core::testing::{FakeClock, Recorded, RecordingPaster};
use skills_pad_core::{Core, CoreConfig, SettingsError};
use tempfile::TempDir;

const READY: &str = ".scratch/app/issues/02-ready.md";
const BLOCKED: &str = ".scratch/app/issues/03-blocked.md";
const DONE: &str = ".scratch/app/issues/01-done.md";

fn write(project: &Path, id: &str, markdown: &str) {
    let path = project.join(id);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, markdown).unwrap();
}

struct World {
    _home: TempDir,
    _project: TempDir,
    core: Core,
    paster: Arc<RecordingPaster>,
}

fn world() -> World {
    let home = TempDir::new().unwrap();
    let project = TempDir::new().unwrap();
    write(
        project.path(),
        DONE,
        "# 01: Done\n\n**Status:** done\n\n**GitHub:** #2\n",
    );
    write(
        project.path(),
        READY,
        "# 02: Ready\n\n**Status:** ready-for-agent\n\n**GitHub:** #3\n",
    );
    write(
        project.path(),
        BLOCKED,
        "# 03: Blocked\n\n**Blocked by:** #3\n\n**Status:** ready-for-agent\n",
    );
    let paster = Arc::new(RecordingPaster::new());
    let core = Core::start(CoreConfig {
        home: home.path().to_path_buf(),
        paster: paster.clone(),
        clock: Arc::new(FakeClock::new()),
    })
    .unwrap();
    {
        let response = ureq::post(&core.url("/hooks/claude"))
            .timeout(Duration::from_secs(1))
            .send_json(json!({
                "session_id": "claude-1",
                "cwd": project.path(),
                "hook_event_name": "SessionStart",
                "source": "startup",
            }))
            .unwrap();
        assert_eq!(response.status(), 204);
    }
    World {
        _home: home,
        _project: project,
        core,
        paster,
    }
}

fn implement_url(base: &str, session: &str, ticket: &str) -> String {
    format!(
        "{base}/sessions/{session}/tickets/{}/implement",
        utf8_percent_encode(ticket, NON_ALPHANUMERIC)
    )
}

fn post(url: &str, cookie: Option<&str>) -> (u16, Value) {
    let mut request = ureq::post(url).timeout(Duration::from_secs(5));
    if let Some(cookie) = cookie {
        request = request.set("Cookie", cookie);
    }
    match request.call() {
        Ok(r) => (r.status(), r.into_json().unwrap()),
        Err(ureq::Error::Status(code, r)) => (code, r.into_json().unwrap_or(Value::Null)),
        Err(e) => panic!("{e}"),
    }
}

#[test]
fn implement_pastes_the_default_template_and_presses_enter() {
    let w = world();

    let (status, body) = post(&implement_url(&w.core.url(""), "claude-1", READY), None);

    assert_eq!(status, 200);
    let text = format!("Implement the ticket at {READY}");
    assert_eq!(body["pasted"], text.as_str());
    assert_eq!(w.paster.events(), [Recorded::Paste(text), Recorded::Enter]);
}

#[test]
fn a_phone_can_implement_over_the_sharing_server() {
    let w = world();
    w.core
        .set_sharing_address_override(Some(Ipv4Addr::LOCALHOST.into()));
    let info = w.core.start_sharing().unwrap();
    let pairing = ureq::get(&info.url).call().unwrap();
    let cookie = pairing
        .all("Set-Cookie")
        .iter()
        .map(|c| c.split(';').next().unwrap())
        .collect::<Vec<_>>()
        .join("; ");
    let base = format!("http://{}", SocketAddr::new(info.address, info.port));

    let (status, _) = post(&implement_url(&base, "claude-1", READY), Some(&cookie));
    assert_eq!(status, 200);
    let (status, _) = post(&implement_url(&base, "claude-1", READY), None);
    assert_eq!(status, 401);
    assert_eq!(w.paster.pasted().len(), 1);
}

#[test]
fn blocked_done_and_unknown_tickets_are_refused() {
    let w = world();
    let base = w.core.url("");

    assert_eq!(
        post(&implement_url(&base, "claude-1", BLOCKED), None).0,
        409
    );
    assert_eq!(post(&implement_url(&base, "claude-1", DONE), None).0, 409);
    assert_eq!(
        post(
            &implement_url(&base, "claude-1", ".scratch/app/issues/99-nope.md"),
            None
        )
        .0,
        404
    );
    assert_eq!(
        post(&implement_url(&base, "claude-1", "../../etc/passwd"), None).0,
        404
    );
    assert_eq!(post(&implement_url(&base, "other", READY), None).0, 404);
    assert!(w.paster.events().is_empty());
}

#[test]
fn the_template_is_per_harness_editable_and_persisted() {
    let w = world();
    assert_eq!(
        w.core
            .set_implement_template("claude", Some("no placeholder")),
        Err(SettingsError::TemplateWithoutPath)
    );
    w.core
        .set_implement_template("claude", Some("/implement {path}"))
        .unwrap();
    assert_eq!(
        w.core.implement_template("codex"),
        "Implement the ticket at {path}"
    );

    let text = w.core.implement_ticket("claude-1", READY).unwrap();
    assert_eq!(text, format!("/implement {READY}"));

    w.core.set_implement_template("claude", None).unwrap();
    assert_eq!(
        w.core.implement_template("claude"),
        "Implement the ticket at {path}"
    );
}

#[test]
fn a_phone_cannot_see_or_implement_tickets_of_sessions_outside_its_view() {
    let w = world();
    w.core
        .set_sharing_address_override(Some(Ipv4Addr::LOCALHOST.into()));
    let info = w.core.start_sharing().unwrap();
    let pairing = ureq::get(&info.url).call().unwrap();
    let cookies: Vec<String> = pairing
        .all("Set-Cookie")
        .iter()
        .map(|c| c.split(';').next().unwrap().to_string())
        .collect();
    let device = cookies
        .iter()
        .find_map(|c| c.strip_prefix("skills_pad_device="))
        .unwrap()
        .to_string();
    let cookie = cookies.join("; ");
    let base = format!("http://{}", SocketAddr::new(info.address, info.port));
    w.core
        .set_phone_view(&device, Some(vec!["h:codex".into()]))
        .unwrap();

    let list = ureq::get(&format!("{base}/sessions/claude-1/tickets"))
        .set("Cookie", &cookie)
        .call();
    assert!(matches!(list, Err(ureq::Error::Status(403, _))));
    assert_eq!(
        post(&implement_url(&base, "claude-1", READY), Some(&cookie)).0,
        403
    );
    assert!(w.paster.events().is_empty());
}
