//! A session's subagents are tracked as a tree, for Claude Code and Codex.

use std::sync::Arc;
use std::time::Duration;

use agent_pad_core::testing::{FakeClock, RecordingPaster};
use agent_pad_core::{AgentNode, Core, CoreConfig, Session, SessionStatus::*};
use serde_json::{json, Value};
use tempfile::TempDir;

struct World {
    _home: TempDir,
    core: Core,
    clock: Arc<FakeClock>,
}

fn world() -> World {
    let home = TempDir::new().unwrap();
    let clock = Arc::new(FakeClock::new());
    let core = Core::start(CoreConfig {
        home: home.path().to_path_buf(),
        paster: Arc::new(RecordingPaster::new()),
        clock: clock.clone(),
    })
    .unwrap();
    World { _home: home, core, clock }
}

fn hook(core: &Core, harness: &str, event: &str, fields: Value) {
    let mut body = json!({"session_id": "s1", "cwd": "/work/project", "hook_event_name": event});
    body.as_object_mut().unwrap().extend(fields.as_object().unwrap().clone());
    let response = ureq::post(&core.url(&format!("/hooks/{harness}")))
        .timeout(Duration::from_secs(1))
        .send_json(body)
        .unwrap();
    assert_eq!(response.status(), 204);
}

fn session(core: &Core) -> Session {
    let listed: Vec<Session> = ureq::get(&core.url("/sessions"))
        .timeout(Duration::from_secs(1))
        .call()
        .unwrap()
        .into_json()
        .unwrap();
    assert_eq!(listed.len(), 1);
    listed.into_iter().next().unwrap()
}

fn spawn_tool(core: &Core, harness: &str, by: Option<&str>) {
    let mut fields = json!({"tool_name": "Task"});
    if let Some(id) = by {
        fields["agent_id"] = id.into();
    }
    hook(core, harness, "PreToolUse", fields);
}

fn start_agent(core: &Core, harness: &str, id: &str, kind: &str) {
    hook(core, harness, "SubagentStart", json!({"agent_id": id, "agent_type": kind}));
}

fn names(nodes: &[AgentNode]) -> Vec<&str> {
    nodes.iter().map(|node| node.agent_id.as_str()).collect()
}

#[test]
fn a_session_without_subagents_has_an_empty_tree() {
    let w = world();
    hook(&w.core, "claude", "SessionStart", json!({"source": "startup"}));
    assert!(session(&w.core).agents.is_empty());
}

#[test]
fn subagents_spawned_by_the_main_agent_are_its_children() {
    let w = world();
    hook(&w.core, "claude", "SessionStart", json!({"source": "startup"}));
    spawn_tool(&w.core, "claude", None);
    start_agent(&w.core, "claude", "a1", "Explore");
    spawn_tool(&w.core, "claude", None);
    start_agent(&w.core, "claude", "a2", "Plan");

    let s = session(&w.core);

    assert_eq!(names(&s.agents), ["a1", "a2"]);
    assert_eq!(s.agents[0].agent_type.as_deref(), Some("Explore"));
    assert_eq!(s.agents[0].status, Running);
    assert!(s.agents[0].children.is_empty());
}

#[test]
fn a_subagent_that_spawns_another_becomes_its_parent() {
    let w = world();
    hook(&w.core, "claude", "SessionStart", json!({"source": "startup"}));
    spawn_tool(&w.core, "claude", None);
    start_agent(&w.core, "claude", "a1", "general-purpose");
    // a1 calls the spawn tool, so the next subagent hangs under a1.
    spawn_tool(&w.core, "claude", Some("a1"));
    start_agent(&w.core, "claude", "a2", "Explore");

    let s = session(&w.core);

    assert_eq!(names(&s.agents), ["a1"]);
    assert_eq!(names(&s.agents[0].children), ["a2"]);
}

#[test]
fn tool_calls_and_blocked_status_are_attributed_to_the_subagent() {
    let w = world();
    hook(&w.core, "claude", "SessionStart", json!({"source": "startup"}));
    spawn_tool(&w.core, "claude", None);
    start_agent(&w.core, "claude", "a1", "Explore");
    hook(&w.core, "claude", "PreToolUse", json!({"tool_name": "Read", "agent_id": "a1"}));
    hook(&w.core, "claude", "PreToolUse", json!({"tool_name": "Grep", "agent_id": "a1"}));
    hook(&w.core, "claude", "PermissionRequest", json!({"agent_id": "a1"}));

    let s = session(&w.core);

    assert_eq!(s.agents[0].tool_count, 2);
    assert_eq!(s.agents[0].status, Blocked);
    assert_eq!(s.tool_count, 3); // the spawn call plus both subagent calls
}

#[test]
fn a_subagent_stop_finishes_only_that_node_and_never_the_session_turn() {
    let w = world();
    hook(&w.core, "claude", "SessionStart", json!({"source": "startup"}));
    hook(&w.core, "claude", "UserPromptSubmit", json!({}));
    spawn_tool(&w.core, "claude", None);
    start_agent(&w.core, "claude", "a1", "Explore");
    w.clock.advance(Duration::from_secs(7));
    hook(&w.core, "claude", "SubagentStop", json!({"agent_id": "a1"}));
    hook(&w.core, "claude", "Stop", json!({"agent_id": "a1"}));

    let s = session(&w.core);

    assert_eq!(s.status, Running);
    assert_eq!(s.agents[0].status, Done);
    assert_eq!(s.agents[0].elapsed_seconds, 7);
    w.clock.advance(Duration::from_secs(30));
    assert_eq!(session(&w.core).agents[0].elapsed_seconds, 7); // stops counting once done
}

#[test]
fn the_main_agents_stop_finishes_every_subagent_and_the_next_prompt_clears_them() {
    let w = world();
    hook(&w.core, "claude", "SessionStart", json!({"source": "startup"}));
    spawn_tool(&w.core, "claude", None);
    start_agent(&w.core, "claude", "a1", "Explore");
    hook(&w.core, "claude", "Stop", json!({}));

    let s = session(&w.core);
    assert_eq!(s.status, Done);
    assert_eq!(s.agents[0].status, Done);

    hook(&w.core, "claude", "UserPromptSubmit", json!({}));
    assert!(session(&w.core).agents.is_empty());
}

#[test]
fn a_subagent_event_without_a_start_still_gets_a_node_under_the_main_agent() {
    let w = world();
    hook(&w.core, "claude", "SessionStart", json!({"source": "startup"}));
    hook(&w.core, "claude", "PreToolUse", json!({"tool_name": "Read", "agent_id": "late", "agent_type": "Explore"}));

    let s = session(&w.core);

    assert_eq!(names(&s.agents), ["late"]);
    assert_eq!(s.agents[0].tool_count, 1);
}

#[test]
fn codex_subagents_form_the_same_tree() {
    let w = world();
    hook(&w.core, "codex", "SessionStart", json!({"source": "startup"}));
    hook(&w.core, "codex", "PreToolUse", json!({"tool_name": "spawn_agent"}));
    start_agent(&w.core, "codex", "worker-1", "worker");
    hook(&w.core, "codex", "PreToolUse", json!({"tool_name": "spawn_agent", "agent_id": "worker-1"}));
    start_agent(&w.core, "codex", "explorer-1", "explorer");

    let s = session(&w.core);

    assert_eq!(names(&s.agents), ["worker-1"]);
    assert_eq!(names(&s.agents[0].children), ["explorer-1"]);
    assert_eq!(s.agents[0].children[0].agent_type.as_deref(), Some("explorer"));
}

#[test]
fn installing_hooks_subscribes_to_subagent_events() {
    let w = world();
    let claude = w.core.preview_claude_hooks().unwrap();
    assert!(claude.after.contains("SubagentStart") && claude.after.contains("SubagentStop"));
    let codex = w.core.preview_codex_hooks().unwrap();
    assert!(codex.after.contains("SubagentStart") && codex.after.contains("SubagentStop"));
}
