//! Ticket 13: Claude Code session status and consent to install hooks.

use std::fs;
use std::net::{Ipv4Addr, SocketAddr, UdpSocket};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use serde_json::{json, Value};
use agent_pad_core::testing::{FakeClock, RecordingPaster};
use agent_pad_core::{
    ClaudeHooksPreview, Core, CoreConfig, Session, SessionStatus, CLAUDE_HOOKS_RESTART_NOTICE,
};
use tempfile::TempDir;

fn start(home: &Path, clock: Arc<FakeClock>) -> Core {
    Core::start(CoreConfig {
        home: home.to_path_buf(),
        paster: Arc::new(RecordingPaster::new()),
        clock,
    })
    .unwrap()
}

fn payload(id: &str, event: &str, fields: Value) -> Value {
    // Claude Code command-hook stdin schema: https://code.claude.com/docs/en/hooks
    let mut body = json!({
        "session_id": id,
        "cwd": "/work/project",
        "transcript_path": "/home/user/.claude/projects/project/session.jsonl",
        "hook_event_name": event,
    });
    body.as_object_mut()
        .unwrap()
        .extend(fields.as_object().unwrap().clone());
    body
}

fn hook(core: &Core, id: &str, event: &str, fields: Value) {
    let response = ureq::post(&core.url("/hooks/claude"))
        .timeout(Duration::from_secs(1))
        .send_json(payload(id, event, fields))
        .unwrap();
    assert_eq!(response.status(), 204);
}

fn sessions(core: &Core) -> Vec<Session> {
    ureq::get(&core.url("/sessions"))
        .timeout(Duration::from_secs(1))
        .call()
        .unwrap()
        .into_json()
        .unwrap()
}

fn status(core: &Core, expected: SessionStatus, tool_count: u64) {
    let listed = sessions(core);
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].status, expected);
    assert_eq!(listed[0].tool_count, tool_count);
}

#[test]
fn claude_hook_events_update_the_session_and_count_each_tool_once() {
    use SessionStatus::*;

    let home = TempDir::new().unwrap();
    let clock = Arc::new(FakeClock::new());
    let core = start(home.path(), clock.clone());

    hook(&core, "one", "SessionStart", json!({"source": "startup"}));
    status(&core, Idle, 0);
    hook(&core, "one", "UserPromptSubmit", json!({"prompt": "Hello"}));
    status(&core, Working, 0);
    hook(&core, "one", "Stop", json!({"stop_hook_active": false}));
    status(&core, Idle, 0);
    hook(
        &core,
        "one",
        "PreToolUse",
        json!({"tool_name": "Bash", "tool_input": {}, "tool_use_id": "toolu_1"}),
    );
    status(&core, Working, 1);
    hook(
        &core,
        "one",
        "PermissionRequest",
        json!({"tool_name": "Bash", "tool_input": {}}),
    );
    status(&core, NeedsInput, 1);
    hook(
        &core,
        "one",
        "PostToolUse",
        json!({"tool_name": "Bash", "tool_input": {}, "tool_use_id": "toolu_1", "tool_response": {}}),
    );
    status(&core, Working, 1);
    hook(
        &core,
        "one",
        "Notification",
        json!({"message": "Waiting", "notification_type": "idle_prompt"}),
    );
    status(&core, Working, 1);
    hook(
        &core,
        "one",
        "Notification",
        json!({"message": "Allow?", "notification_type": "permission_prompt"}),
    );
    status(&core, NeedsInput, 1);
    hook(
        &core,
        "one",
        "PostToolUseFailure",
        json!({"tool_name": "Bash", "tool_input": {}, "error": "Interrupted", "is_interrupt": true}),
    );
    status(&core, Idle, 1);

    clock.advance(Duration::from_secs(42));
    hook(&core, "one", "SessionEnd", json!({"reason": "other"}));
    clock.advance(Duration::from_secs(59));
    hook(
        &core,
        "one",
        "PostToolUse",
        json!({"tool_use_id": "toolu_1"}),
    );
    let expected = Session {
        session_id: "one".into(),
        harness: "claude".into(),
        project_folder: PathBuf::from("/work/project"),
        status: Ended,
        elapsed_seconds: 42,
        tool_count: 1,
    };
    assert_eq!(sessions(&core), vec![expected.clone()]);
    assert_eq!(core.list_sessions(), vec![expected]);
    clock.advance(Duration::from_secs(1));
    assert!(sessions(&core).is_empty());
}

#[test]
fn a_working_session_becomes_stuck_at_the_configured_threshold() {
    use SessionStatus::*;

    let home = TempDir::new().unwrap();
    let clock = Arc::new(FakeClock::new());
    let core = start(home.path(), clock.clone());
    hook(&core, "one", "SessionStart", json!({"source": "startup"}));
    hook(&core, "one", "UserPromptSubmit", json!({"prompt": "Hello"}));

    clock.advance(Duration::from_secs(299));
    status(&core, Working, 0);
    clock.advance(Duration::from_secs(1));
    status(&core, Stuck, 0);
    hook(
        &core,
        "one",
        "PreToolUse",
        json!({"tool_use_id": "toolu_1"}),
    );
    status(&core, Working, 1);

    core.set_stuck_threshold(Duration::from_secs(10));
    clock.advance(Duration::from_secs(9));
    status(&core, Working, 1);
    clock.advance(Duration::from_secs(1));
    status(&core, Stuck, 1);
    hook(
        &core,
        "one",
        "PermissionRequest",
        json!({"tool_name": "Bash"}),
    );
    clock.advance(Duration::from_secs(600));
    status(&core, NeedsInput, 1);
    hook(&core, "one", "Stop", json!({"stop_hook_active": false}));
    clock.advance(Duration::from_secs(600));
    status(&core, Idle, 1);
}

#[test]
fn sessions_already_running_are_hidden_until_restarted() {
    let home = TempDir::new().unwrap();
    let core = start(home.path(), Arc::new(FakeClock::new()));
    hook(&core, "old", "UserPromptSubmit", json!({"prompt": "Hello"}));
    hook(&core, "old", "SessionStart", json!({"source": "compact"}));
    hook(
        &core,
        "old",
        "PreToolUse",
        json!({"tool_use_id": "toolu_1"}),
    );
    assert!(sessions(&core).is_empty());
    assert!(CLAUDE_HOOKS_RESTART_NOTICE.contains("Restart Claude Code sessions"));

    hook(&core, "old", "SessionStart", json!({"source": "resume"}));
    hook(&core, "new", "SessionStart", json!({"source": "startup"}));
    hook(
        &core,
        "old",
        "UserPromptSubmit",
        json!({"prompt": "Hello", "cwd": "/work/other"}),
    );
    let listed = sessions(&core);
    assert_eq!(listed.len(), 2);
    assert_eq!(listed[0].session_id, "new");
    assert_eq!(listed[0].status, SessionStatus::Idle);
    assert_eq!(listed[1].session_id, "old");
    assert_eq!(listed[1].project_folder, PathBuf::from("/work/other"));
    assert_eq!(listed[1].status, SessionStatus::Working);
    assert_eq!(listed[1].tool_count, 0);
}

#[test]
fn only_the_local_api_accepts_hook_events_and_paired_phones_can_list_sessions() {
    let home = TempDir::new().unwrap();
    let core = start(home.path(), Arc::new(FakeClock::new()));
    assert_eq!(core.addr().ip(), Ipv4Addr::LOCALHOST);

    let socket = UdpSocket::bind("0.0.0.0:0").unwrap();
    if socket.connect("192.0.2.1:80").is_ok() {
        let lan = socket.local_addr().unwrap().ip();
        if !lan.is_loopback() {
            let local_api_on_lan = SocketAddr::new(lan, core.addr().port());
            let result = ureq::post(&format!("http://{local_api_on_lan}/hooks/claude"))
                .timeout(Duration::from_secs(1))
                .send_json(payload("lan", "SessionStart", json!({"source": "startup"})));
            assert!(
                matches!(result, Err(ureq::Error::Transport(_))),
                "{result:?}"
            );
        }
    }

    // Peer identity comes from the socket, not forwarding headers.
    ureq::post(&core.url("/hooks/claude"))
        .timeout(Duration::from_secs(1))
        .set("X-Forwarded-For", "192.0.2.1")
        .send_json(payload(
            "local",
            "SessionStart",
            json!({"source": "startup"}),
        ))
        .unwrap();
    status(&core, SessionStatus::Idle, 0);

    core.set_sharing_address_override(Some(Ipv4Addr::LOCALHOST.into()));
    let sharing = core.start_sharing().unwrap();
    let phone = format!("http://{}", SocketAddr::new(sharing.address, sharing.port));
    let (_, query) = sharing.url.split_once('?').unwrap();
    let paired = ureq::get(&sharing.url)
        .timeout(Duration::from_secs(1))
        .call()
        .unwrap();
    let cookie = &paired
        .all("Set-Cookie")
        .iter()
        .map(|c| c.split(';').next().unwrap())
        .collect::<Vec<_>>()
        .join("; ");
    let listed: Vec<Session> = ureq::get(&format!("{phone}/sessions"))
        .timeout(Duration::from_secs(1))
        .set("Cookie", cookie)
        .call()
        .unwrap()
        .into_json()
        .unwrap();
    assert_eq!(listed, sessions(&core));

    let unpaired = ureq::get(&format!("{phone}/sessions"))
        .timeout(Duration::from_secs(1))
        .call();
    assert!(matches!(unpaired, Err(ureq::Error::Status(401, _))));
    let phone_post = ureq::post(&format!("{phone}/hooks/claude?{query}"))
        .timeout(Duration::from_secs(1))
        .send_json(payload(
            "phone",
            "SessionStart",
            json!({"source": "startup"}),
        ));
    assert!(matches!(phone_post, Err(ureq::Error::Status(404, _))));
    assert_eq!(sessions(&core), listed);
}

#[test]
fn malformed_hook_json_is_rejected() {
    let home = TempDir::new().unwrap();
    let core = start(home.path(), Arc::new(FakeClock::new()));
    let result = ureq::post(&core.url("/hooks/claude"))
        .timeout(Duration::from_secs(1))
        .set("Content-Type", "application/json")
        .send_string(r#"{"hook_event_name":"SessionStart"}"#);
    assert!(matches!(result, Err(ureq::Error::Status(400, _))));
    assert!(sessions(&core).is_empty());
}

#[test]
fn preview_leaves_settings_untouched_and_removal_restores_the_original_bytes() {
    let home = TempDir::new().unwrap();
    let core = start(home.path(), Arc::new(FakeClock::new()));
    let path = home.path().join(".claude/settings.json");
    fs::create_dir(path.parent().unwrap()).unwrap();
    let original = r#"{ "model": "test", "hooks": {"Stop": [{"hooks":[{"type":"command","command":"other-hook"}]}], "Notification": []} }"#;
    fs::write(&path, original).unwrap();
    let home_entries = fs::read_dir(home.path()).unwrap().count();

    let preview = core.preview_claude_hooks().unwrap();
    let preview: ClaudeHooksPreview = serde_json::from_value(json!(preview)).unwrap();
    assert_eq!(preview.path, path);
    assert_eq!(preview.before.as_deref(), Some(original));
    assert_eq!(fs::read_to_string(&path).unwrap(), original);
    assert_eq!(fs::read_dir(home.path()).unwrap().count(), home_entries);
    core.apply_claude_hooks(&preview).unwrap();
    assert_eq!(fs::read_to_string(&path).unwrap(), preview.after);
    let installed: Value = serde_json::from_str(&preview.after).unwrap();
    assert_eq!(installed["model"], "test");
    assert_eq!(
        installed["hooks"]["Stop"][0]["hooks"][0]["command"],
        "other-hook"
    );
    assert_eq!(
        installed["hooks"]["Notification"][0]["matcher"],
        "permission_prompt"
    );
    let already_installed = core.preview_claude_hooks().unwrap();
    assert_eq!(
        already_installed.before.as_deref(),
        Some(preview.after.as_str())
    );
    assert_eq!(already_installed.after, preview.after);
    core.apply_claude_hooks(&already_installed).unwrap();

    core.remove_claude_hooks().unwrap();
    core.remove_claude_hooks().unwrap();
    assert_eq!(
        fs::read_to_string(path).unwrap().as_bytes(),
        original.as_bytes()
    );
}

#[test]
fn apply_rejects_a_stale_preview_and_removal_preserves_subsequent_user_edits() {
    let home = TempDir::new().unwrap();
    let core = start(home.path(), Arc::new(FakeClock::new()));
    let preview = core.preview_claude_hooks().unwrap();
    assert!(preview.before.is_none());
    assert_eq!(fs::read_dir(home.path()).unwrap().count(), 0);

    fs::create_dir(preview.path.parent().unwrap()).unwrap();
    let original = json!({"theme": "dark", "hooks": {"Stop": []}});
    fs::write(&preview.path, original.to_string()).unwrap();
    assert!(core.apply_claude_hooks(&preview).is_err());
    assert_eq!(
        fs::read_to_string(&preview.path).unwrap(),
        original.to_string()
    );

    let preview = core.preview_claude_hooks().unwrap();
    core.apply_claude_hooks(&preview).unwrap();
    let mut edited: Value = serde_json::from_str(&preview.after).unwrap();
    edited["theme"] = json!("light");
    let other = json!({"type": "command", "command": "new-user-hook"});
    edited["hooks"]["PreToolUse"][0]["hooks"]
        .as_array_mut()
        .unwrap()
        .push(other.clone());
    fs::write(&preview.path, edited.to_string()).unwrap();
    assert_eq!(
        core.preview_claude_hooks().unwrap().after,
        edited.to_string()
    );

    core.remove_claude_hooks().unwrap();
    let removed: Value = serde_json::from_str(&fs::read_to_string(preview.path).unwrap()).unwrap();
    assert_eq!(
        removed,
        json!({"theme": "light", "hooks": {"Stop": [], "PreToolUse": [{"hooks": [other]}]}})
    );
}

#[cfg(unix)]
#[test]
fn the_installed_command_forwards_to_the_current_port_and_succeeds_offline() {
    use std::io::Write;
    use std::process::{Command, Stdio};

    fn run(command: &str, id: &str) {
        let mut child = Command::new("/bin/sh")
            .args(["-c", command])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let mut input = child.stdin.take().unwrap();
        write!(
            input,
            "{}",
            payload(id, "SessionStart", json!({"source": "startup"}))
        )
        .unwrap();
        drop(input);
        let output = child.wait_with_output().unwrap();
        assert!(output.status.success());
        assert!(output.stdout.is_empty());
        assert!(output.stderr.is_empty());
    }

    let temp = TempDir::new().unwrap();
    let home = temp.path().join("Claude user's home");
    fs::create_dir(&home).unwrap();
    let clock = Arc::new(FakeClock::new());
    let core = start(&home, clock.clone());
    let preview = core.preview_claude_hooks().unwrap();
    core.apply_claude_hooks(&preview).unwrap();
    let settings: Value = serde_json::from_str(&preview.after).unwrap();
    let command = settings["hooks"]["SessionStart"][0]["hooks"][0]["command"]
        .as_str()
        .unwrap();
    run(command, "first");
    assert_eq!(sessions(&core)[0].session_id, "first");

    drop(core);
    run(command, "offline");
    let core = start(&home, clock);
    run(command, "second");
    assert_eq!(sessions(&core)[0].session_id, "second");

    core.remove_claude_hooks().unwrap();
    assert!(!preview.path.exists());
    run(command, "removed");
    assert_eq!(sessions(&core).len(), 1);
}

#[test]
fn the_stuck_threshold_is_saved_across_restarts() {
    let home = tempfile::TempDir::new().unwrap();
    let start = || {
        agent_pad_core::Core::start(agent_pad_core::CoreConfig {
            home: home.path().to_path_buf(),
            paster: std::sync::Arc::new(agent_pad_core::testing::RecordingPaster::new()),
            clock: std::sync::Arc::new(agent_pad_core::testing::FakeClock::new()),
        })
        .unwrap()
    };
    assert_eq!(start().stuck_threshold(), Duration::from_secs(300));
    start().set_stuck_threshold(Duration::from_secs(900));
    assert_eq!(start().stuck_threshold(), Duration::from_secs(900));
}
