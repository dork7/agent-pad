//! Ticket 24: Run only pastes after the hook-owned session window was raised.

use std::path::Path;
use std::sync::{Arc, Mutex};

use agent_pad_core::testing::{FakeClock, Recorded, RecordingPaster};
use agent_pad_core::{Core, CoreConfig, SessionWindow, WindowTarget};
use serde_json::json;
use tempfile::TempDir;

#[derive(Default)]
struct RecordingWindow {
    targets: Mutex<Vec<WindowTarget>>,
    found: bool,
}

impl SessionWindow for RecordingWindow {
    fn raise(&self, target: &WindowTarget) -> bool {
        self.targets.lock().unwrap().push(target.clone());
        self.found
    }
}

fn hook(core: &Core, project: &Path, process: Option<u32>) {
    let mut request = ureq::post(&core.url("/hooks/claude"));
    if let Some(process) = process {
        request = request.set("X-Agent-Pad-Process-Id", &process.to_string());
    }
    assert_eq!(
        request
            .send_json(json!({
                "session_id": "session-one", "cwd": project,
                "hook_event_name": "SessionStart", "source": "startup"
            }))
            .unwrap()
            .status(),
        204
    );
}

fn world(
    found: bool,
    process: Option<u32>,
) -> (
    TempDir,
    TempDir,
    Core,
    Arc<RecordingPaster>,
    Arc<RecordingWindow>,
    String,
) {
    let home = TempDir::new().unwrap();
    let project = TempDir::new().unwrap();
    let skill = home.path().join(".claude/skills/review/SKILL.md");
    std::fs::create_dir_all(skill.parent().unwrap()).unwrap();
    std::fs::write(&skill, "---\nname: review\ndescription: Review\n---\n").unwrap();
    let paster = Arc::new(RecordingPaster::new());
    let core = Core::start(CoreConfig {
        home: home.path().into(),
        paster: paster.clone(),
        clock: Arc::new(FakeClock::new()),
    })
    .unwrap();
    let window = Arc::new(RecordingWindow {
        found,
        ..Default::default()
    });
    core.set_session_window(window.clone());
    hook(&core, project.path(), process);
    let skill_id = core
        .list_skills()
        .into_iter()
        .find(|s| s.name == "review")
        .unwrap()
        .id;
    (home, project, core, paster, window, skill_id)
}

#[test]
fn run_raises_hook_process_window_before_paste_and_enter() {
    let (_home, project, core, paster, window, skill_id) = world(true, Some(4242));
    core.run_skill_in_session("session-one", &skill_id).unwrap();
    assert_eq!(
        window.targets.lock().unwrap().as_slice(),
        &[WindowTarget {
            process_id: 4242,
            project_folder: project.path().into(),
        }]
    );
    assert_eq!(
        paster.events(),
        [Recorded::Paste("/review ".into()), Recorded::Enter]
    );
}

#[test]
fn missing_or_unmatched_window_never_pastes() {
    for (found, process) in [(false, Some(4242)), (true, None)] {
        let (_home, _project, core, paster, window, skill_id) = world(found, process);
        assert_eq!(
            core.run_skill_in_session("session-one", &skill_id)
                .unwrap_err()
                .to_string(),
            "session window was not found"
        );
        assert!(paster.events().is_empty());
        assert_eq!(
            window.targets.lock().unwrap().len(),
            usize::from(process.is_some())
        );
    }
}

#[test]
fn unknown_session_and_skill_do_not_raise_or_paste() {
    let (_home, _project, core, paster, window, skill_id) = world(true, Some(4242));
    assert!(core.run_skill_in_session("other", &skill_id).is_err());
    assert!(core.run_skill_in_session("session-one", "unknown").is_err());
    assert!(window.targets.lock().unwrap().is_empty());
    assert!(paster.events().is_empty());
}

#[test]
fn run_api_reports_missing_window_without_pasting() {
    let (_home, _project, core, paster, _window, skill_id) = world(false, Some(4242));
    let response = ureq::post(&core.url("/sessions/session-one/run"))
        .send_json(json!({"id": skill_id, "text": "arbitrary phone text"}))
        .unwrap_err();
    let ureq::Error::Status(status, response) = response else {
        panic!("expected HTTP refusal")
    };
    assert_eq!(status, 409);
    let body: serde_json::Value = response.into_json().unwrap();
    assert_eq!(body["error"], "session window was not found");
    assert!(paster.events().is_empty());
}
