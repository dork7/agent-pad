//! Ticket 14: Codex hooks, consent, and phone session views.

use std::fs;
use std::net::Ipv4Addr;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use serde_json::{json, Value};
use agent_pad_core::testing::{FakeClock, RecordingPaster};
use agent_pad_core::{Core, CoreConfig, Session, SessionStatus};
use tempfile::TempDir;

fn start(home: &Path, clock: Arc<FakeClock>) -> Core {
    Core::start(CoreConfig {
        home: home.to_path_buf(),
        paster: Arc::new(RecordingPaster::new()),
        clock,
    })
    .unwrap()
}

fn payload(event: &str, fields: Value) -> Value {
    // Command-hook stdin schema: https://developers.openai.com/codex/hooks
    let mut body = json!({
        "session_id": "same-id",
        "cwd": "/work/project",
        "hook_event_name": event,
        "transcript_path": null,
        "model": "codex-test",
        "permission_mode": "default",
    });
    body.as_object_mut()
        .unwrap()
        .extend(fields.as_object().unwrap().clone());
    body
}

fn hook(core: &Core, harness: &str, event: &str, fields: Value) {
    let response = ureq::post(&core.url(&format!("/hooks/{harness}")))
        .timeout(Duration::from_secs(2))
        .send_json(payload(event, fields))
        .unwrap();
    assert_eq!(response.status(), 204);
}

fn sessions(core: &Core) -> Vec<Session> {
    ureq::get(&core.url("/sessions"))
        .timeout(Duration::from_secs(2))
        .call()
        .unwrap()
        .into_json()
        .unwrap()
}

#[test]
fn codex_hooks_track_permission_requests_interrupts_and_session_end() {
    use SessionStatus::*;

    let home = TempDir::new().unwrap();
    let clock = Arc::new(FakeClock::new());
    let core = start(home.path(), clock.clone());

    hook(
        &core,
        "codex",
        "UserPromptSubmit",
        json!({"prompt": "Hello"}),
    );
    hook(&core, "codex", "SessionStart", json!({"source": "compact"}));
    assert!(sessions(&core).is_empty());

    let status = |expected, tool_count| {
        let listed = sessions(&core);
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].status, expected);
        assert_eq!(listed[0].tool_count, tool_count);
    };
    hook(&core, "codex", "SessionStart", json!({"source": "resume"}));
    status(Done, 0);
    hook(
        &core,
        "codex",
        "UserPromptSubmit",
        json!({"turn_id": "one", "prompt": "Hello"}),
    );
    status(Running, 0);
    clock.advance(Duration::from_secs(300));
    status(Stalled, 0);

    let tool = json!({
        "turn_id": "one",
        "tool_name": "Bash",
        "tool_input": {"command": "pwd"}
    });
    hook(&core, "codex", "PreToolUse", tool.clone());
    status(Running, 1);
    hook(&core, "codex", "PermissionRequest", tool.clone());
    status(Blocked, 1);
    hook(&core, "codex", "Interrupt", json!({"turn_id": "one"}));
    status(Done, 1);
    hook(&core, "codex", "PreToolUse", tool.clone());
    hook(&core, "codex", "PermissionRequest", tool.clone());
    status(Blocked, 2);
    hook(&core, "codex", "PostToolUse", tool);
    status(Running, 2);
    hook(
        &core,
        "codex",
        "Stop",
        json!({"turn_id": "one", "stop_hook_active": false}),
    );
    status(Done, 2);

    hook(&core, "codex", "SessionEnd", json!({"reason": "other"}));
    assert_eq!(
        sessions(&core),
        [Session {
            session_id: "same-id".into(),
            harness: "codex".into(),
            project_folder: PathBuf::from("/work/project"),
            status: Ended,
            elapsed_seconds: 300,
            last_event_seconds: 0,
            tool_count: 2,
            agents: Vec::new(),
        }]
    );
    clock.advance(Duration::from_secs(600));
    assert!(sessions(&core).is_empty());
}

const CONFIG: &str = r#"# User configuration
model  = 'my-model' # keep this spacing
notify = ['my-notifier']

[features]
hooks = true # enable hooks

[[hooks.PreToolUse]]
matcher = 'Bash' # user matcher

[[hooks.PreToolUse.hooks]]
type = 'command'
command  = 'echo user-hook' # user hook

[projects.'/work/project']
trust_level = 'trusted'
# trailing comment
"#;

fn write_config(home: &Path, text: &str) -> PathBuf {
    let directory = home.join(".codex");
    fs::create_dir_all(&directory).unwrap();
    let path = directory.join("config.toml");
    fs::write(&path, text).unwrap();
    path
}

#[test]
fn codex_consent_rejects_stale_previews_and_restores_original_bytes() {
    let home = TempDir::new().unwrap();
    let path = write_config(home.path(), CONFIG);
    let core = start(home.path(), Arc::new(FakeClock::new()));
    let preview = core.preview_codex_hooks().unwrap();
    assert_eq!(preview.path, path);
    assert_eq!(preview.before.as_deref(), Some(CONFIG));
    assert_eq!(fs::read_to_string(&path).unwrap(), CONFIG);
    assert!(!agent_pad_core::settings::config_dir(home.path()).exists());
    assert!(preview.after.contains("[[hooks.Interrupt.hooks]]"));
    assert!(preview.after.contains("[[hooks.PermissionRequest.hooks]]"));

    let edited =
        format!("{CONFIG}\n# a change made while the preview was open\n").replace('\n', "\r\n");
    fs::write(&path, &edited).unwrap();
    assert!(core.apply_codex_hooks(&preview).is_err());
    assert_eq!(fs::read_to_string(&path).unwrap(), edited);

    let preview = core.preview_codex_hooks().unwrap();
    core.apply_codex_hooks(&preview).unwrap();
    assert_eq!(fs::read_to_string(&path).unwrap(), preview.after);
    assert!(!preview.after.replace("\r\n", "").contains('\n'));
    assert!(preview
        .after
        .contains("command  = 'echo user-hook' # user hook"));
    assert!(preview
        .after
        .contains("model  = 'my-model' # keep this spacing"));
    assert_eq!(core.preview_codex_hooks().unwrap().after, preview.after);

    drop(core);
    let core = start(home.path(), Arc::new(FakeClock::new()));
    core.remove_codex_hooks().unwrap();
    assert_eq!(fs::read(&path).unwrap(), edited.as_bytes());
}

#[test]
fn codex_removal_keeps_other_settings_and_hooks_after_edits() {
    let home = TempDir::new().unwrap();
    // TOML allows both inline arrays and arrays of tables.
    let original = CONFIG.replace(
        "[[hooks.PreToolUse]]",
        "[hooks]\nStop = [ { hooks = [{ type = 'command', command = 'keep-stop' }] } ] # inline hook\n\n[[hooks.PreToolUse]]",
    );
    let path = write_config(home.path(), &original);
    let core = start(home.path(), Arc::new(FakeClock::new()));
    let preview = core.preview_codex_hooks().unwrap();
    core.apply_codex_hooks(&preview).unwrap();

    let new_hook = "\n[[hooks.Interrupt]]\n\n[[hooks.Interrupt.hooks]]\ntype = 'command'\ncommand = 'keep-interrupt' # added later\n";
    let installed = preview
        .after
        .replace("model  = 'my-model'", "model  = 'new-model'");
    fs::write(&path, format!("{installed}{new_hook}")).unwrap();
    core.remove_codex_hooks().unwrap();

    let remaining = fs::read_to_string(&path).unwrap();
    assert!(remaining.contains("model  = 'new-model' # keep this spacing"));
    assert!(remaining.contains("command  = 'echo user-hook' # user hook"));
    assert!(remaining.contains(
        "Stop = [ { hooks = [{ type = 'command', command = 'keep-stop' }] } ] # inline hook"
    ));
    assert!(remaining.contains("command = 'keep-interrupt' # added later"));
    assert!(remaining.contains("# trailing comment"));
    assert!(!remaining.contains("skills-pad-"));
    assert!(!remaining.contains("hooks.PermissionRequest"));
    assert_eq!(
        remaining.parse::<toml_edit::DocumentMut>().unwrap()["hooks"]["Interrupt"]
            .as_array_of_tables()
            .unwrap()
            .len(),
        1
    );
}

#[cfg(unix)]
#[test]
fn installed_command_forwards_stdin_to_the_current_listener() {
    use std::process::Command;

    let home = TempDir::new().unwrap();
    let quoted_home = home.path().join("home's directory");
    let core = start(&quoted_home, Arc::new(FakeClock::new()));
    let preview = core.preview_codex_hooks().unwrap();
    assert!(preview.before.is_none());
    core.apply_codex_hooks(&preview).unwrap();
    drop(core);

    let core = start(&quoted_home, Arc::new(FakeClock::new()));
    let config: toml_edit::DocumentMut =
        fs::read_to_string(&preview.path).unwrap().parse().unwrap();
    let command = config["hooks"]["SessionStart"][0]["hooks"][0]["command"]
        .as_str()
        .unwrap();
    let body_path = home.path().join("stdin.json");
    fs::write(
        &body_path,
        payload("SessionStart", json!({"source": "startup"})).to_string(),
    )
    .unwrap();
    let run = || {
        let output = Command::new("sh")
            .args(["-c", command])
            .stdin(fs::File::open(&body_path).unwrap())
            .output()
            .unwrap();
        assert!(output.status.success());
        assert!(output.stdout.is_empty());
    };

    run();
    assert_eq!(sessions(&core).len(), 1);
    core.remove_codex_hooks().unwrap();
    assert!(!preview.path.exists());
    hook(&core, "codex", "Interrupt", json!({}));
    hook(&core, "codex", "UserPromptSubmit", json!({}));
    run();
    assert_eq!(sessions(&core)[0].status, SessionStatus::Running);
}

#[test]
fn a_paired_phone_filters_sessions_by_its_current_harness_chips() {
    let home = TempDir::new().unwrap();
    let directory = TempDir::new().unwrap();
    let core = start(home.path(), Arc::new(FakeClock::new()));
    let directory_id = core.add_directory(directory.path(), "codex").unwrap();
    for harness in ["claude", "codex"] {
        hook(&core, harness, "SessionStart", json!({"source": "startup"}));
    }
    for harness in ["cursor", "gemini", "opencode", "copilot"] {
        let response = ureq::post(&core.url(&format!("/hooks/{harness}")))
            .send_json(payload("SessionStart", json!({"source": "startup"})));
        assert!(matches!(response, Err(ureq::Error::Status(404, _))));
    }
    assert_eq!(sessions(&core).len(), 2);

    core.set_sharing_address_override(Some(Ipv4Addr::LOCALHOST.into()));
    let sharing = core.start_sharing().unwrap();
    let paired = ureq::get(&sharing.url).call().unwrap();
    let cookie = paired
        .all("Set-Cookie")
        .iter()
        .map(|cookie| cookie.split(';').next().unwrap())
        .collect::<Vec<_>>()
        .join("; ");
    let phone_id = core.list_phones()[0].id.clone();
    let visible = || {
        let sessions: Vec<Session> = ureq::get(&format!(
            "http://{}:{}/sessions",
            sharing.address, sharing.port
        ))
        .set("Cookie", &cookie)
        .timeout(Duration::from_secs(2))
        .call()
        .unwrap()
        .into_json()
        .unwrap();
        sessions.into_iter().map(|s| s.harness).collect::<Vec<_>>()
    };
    assert_eq!(visible(), ["claude", "codex"]);

    core.set_phone_view(&phone_id, Some(vec!["h:codex".into()]))
        .unwrap();
    assert_eq!(visible(), ["codex"]);
    core.set_phone_view(&phone_id, Some(vec!["h:claude".into()]))
        .unwrap();
    assert_eq!(visible(), ["claude"]);
    core.set_phone_view(&phone_id, Some(vec![format!("d:{directory_id}")]))
        .unwrap();
    assert!(visible().is_empty());
    core.set_phone_view(&phone_id, Some(vec![])).unwrap();
    assert!(visible().is_empty());
    core.set_phone_view(&phone_id, None).unwrap();
    assert_eq!(visible(), ["claude", "codex"]);
}
