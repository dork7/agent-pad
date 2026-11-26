//! Ticket 15: optional Enter after every paste.

use std::fs;
use std::path::Path;
use std::sync::Arc;
use std::thread;

use agent_pad_core::settings::config_dir;
use agent_pad_core::testing::{FakeClock, Recorded, RecordingPaster};
use agent_pad_core::{Core, CoreConfig};
use tempfile::TempDir;

fn write_skill(dir: &Path, name: &str) {
    let dir = dir.join(name);
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("SKILL.md"), format!("---\nname: {name}\n---\n")).unwrap();
}

fn home() -> TempDir {
    let home = TempDir::new().unwrap();
    write_skill(&home.path().join(".claude/skills"), "tdd");
    write_skill(&home.path().join(".codex/skills"), "plan");
    home
}

fn start(home: &TempDir, paster: Arc<RecordingPaster>) -> Core {
    Core::start(CoreConfig {
        home: home.path().to_path_buf(),
        paster,
        clock: Arc::new(FakeClock::new()),
    })
    .unwrap()
}

fn paste(text: &str) -> Recorded {
    Recorded::Paste(text.into())
}

#[test]
fn off_by_default_so_only_the_invocation_is_pasted() {
    let home = home();
    let paster = Arc::new(RecordingPaster::new());
    let core = start(&home, paster.clone());

    assert!(!core.settings().send_enter);
    core.paste_skill("claude:tdd").unwrap();

    assert_eq!(paster.events(), [paste("/tdd ")]);
}

#[test]
fn when_on_enter_follows_the_paste_and_applies_to_the_next_tap() {
    let home = home();
    let paster = Arc::new(RecordingPaster::new());
    let core = start(&home, paster.clone());

    core.paste_skill("claude:tdd").unwrap();
    core.set_send_enter(true).unwrap();
    core.paste_skill("codex:plan").unwrap();
    core.set_send_enter(false).unwrap();
    core.paste_skill("claude:tdd").unwrap();

    assert_eq!(
        paster.events(),
        [
            paste("/tdd "),
            paste("$plan "),
            Recorded::Enter,
            paste("/tdd ")
        ]
    );
}

#[test]
fn the_setting_persists_across_restarts_and_keeps_other_fields() {
    let home = home();
    let file = config_dir(home.path()).join("settings.json");
    fs::create_dir_all(file.parent().unwrap()).unwrap();
    fs::write(&file, r#"{"hotkey": "Cmd+Option+S"}"#).unwrap();

    start(&home, Arc::new(RecordingPaster::new()))
        .set_send_enter(true)
        .unwrap();

    let paster = Arc::new(RecordingPaster::new());
    let restarted = start(&home, paster.clone());
    assert!(restarted.settings().send_enter);
    restarted.paste_skill("claude:tdd").unwrap();
    assert_eq!(paster.events(), [paste("/tdd "), Recorded::Enter]);

    let saved: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(file).unwrap()).unwrap();
    assert_eq!(saved["hotkey"], "Cmd+Option+S");
    assert_eq!(saved["send_enter"], true);
}

#[test]
fn concurrent_pastes_each_get_their_own_enter_in_order() {
    let home = home();
    let paster = Arc::new(RecordingPaster::new());
    let core = Arc::new(start(&home, paster.clone()));
    core.set_send_enter(true).unwrap();

    let workers: Vec<_> = ["claude:tdd", "codex:plan"]
        .into_iter()
        .map(|id| {
            let core = core.clone();
            thread::spawn(move || {
                for _ in 0..5 {
                    core.paste_skill(id).unwrap();
                }
            })
        })
        .collect();
    for w in workers {
        w.join().unwrap();
    }

    let events = paster.events();
    assert_eq!(events.len(), 20);
    for pair in events.chunks(2) {
        assert!(matches!(pair[0], Recorded::Paste(_)), "{events:?}");
        assert_eq!(pair[1], Recorded::Enter, "{events:?}");
    }
}
