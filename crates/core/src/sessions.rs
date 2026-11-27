//! In-memory session state. Ended sessions retain their final elapsed time and
//! disappear after ten minutes. A Core restart starts with an empty session list.

use std::collections::{BTreeMap, VecDeque};
use std::path::PathBuf;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

use crate::{Core, State};
use crate::WindowTarget;

const ENDED_RETENTION: Duration = Duration::from_secs(10 * 60);

/// Display after installing Claude Code hooks.
pub const CLAUDE_HOOKS_RESTART_NOTICE: &str =
    "Restart Claude Code sessions started before hooks were installed.";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionStatus {
    Running,
    Blocked,
    Done,
    Ended,
    Stalled,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Session {
    pub session_id: String,
    /// Stable harness ID, as used by skill and harness chips.
    pub harness: String,
    /// Working directory from the most recent recognized event.
    pub project_folder: PathBuf,
    pub status: SessionStatus,
    pub elapsed_seconds: u64,
    /// Time since the latest hook event, for the Session screen.
    #[serde(default)]
    pub last_event_seconds: u64,
    /// Number of tool calls observed at PreToolUse.
    pub tool_count: u64,
    /// Subagents the session spawned, as a tree: each node's `children` are the agents it
    /// spawned in turn. Empty when the session has run no subagents.
    #[serde(default)]
    pub agents: Vec<AgentNode>,
}

/// A subagent in a session's tree.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentNode {
    pub agent_id: String,
    /// The subagent's type or name, such as `Explore`, when the harness reports one.
    pub agent_type: Option<String>,
    pub status: SessionStatus,
    pub elapsed_seconds: u64,
    pub tool_count: u64,
    pub children: Vec<AgentNode>,
}

/// Which agent an event came from. `agent` is `None` for the session's main agent.
#[derive(Debug, Clone, Default)]
pub(crate) struct Actor {
    pub agent: Option<String>,
    pub agent_type: Option<String>,
    /// The event is a tool call that spawns a subagent (`Task`, `Agent` or `spawn_agent`).
    pub spawns_agent: bool,
}

/// Parents are never reported by the harnesses, so a new subagent is attached to whichever agent
/// most recently called a spawn tool that no earlier subagent has claimed.
const MAX_PENDING_SPAWNS: usize = 32;
const MAX_AGENTS: usize = 200;

pub(crate) fn is_spawn_tool(name: &str) -> bool {
    matches!(name, "Task" | "Agent" | "spawn_agent")
}

pub(crate) enum Event {
    Start,
    /// A new user prompt: starts a new turn, so finished subagents are dropped.
    Prompt,
    SubagentStart,
    SubagentStop,
    Working,
    ToolStarted,
    Permission,
    Stop,
    End,
}

struct TrackedSession {
    project_folder: PathBuf,
    process_id: Option<u32>,
    status: SessionStatus,
    started: Instant,
    last_event: Instant,
    tool_count: u64,
    agents: BTreeMap<String, TrackedAgent>,
    pending_spawns: VecDeque<Option<String>>,
}

struct TrackedAgent {
    agent_type: Option<String>,
    /// `None` means spawned by the main agent.
    parent: Option<String>,
    status: SessionStatus,
    started: Instant,
    last_event: Instant,
    tool_count: u64,
}

pub(crate) struct Sessions {
    // Set from settings at start (see `Sessions::with_threshold`).
    by_id: BTreeMap<(&'static str, String), TrackedSession>,
    stuck_threshold: Duration,
}

impl Default for Sessions {
    fn default() -> Self {
        Self {
            by_id: BTreeMap::new(),
            stuck_threshold: Duration::from_secs(5 * 60),
        }
    }
}

impl TrackedSession {
    fn add_agent(&mut self, id: String, agent_type: Option<String>, parent: Option<String>, now: Instant) {
        if self.agents.len() >= MAX_AGENTS {
            // Drop the oldest finished agent; if none is finished, ignore the newcomer.
            let oldest_done = self
                .agents
                .iter()
                .filter(|(_, agent)| agent.status == SessionStatus::Done)
                .min_by_key(|(_, agent)| agent.started)
                .map(|(id, _)| id.clone());
            match oldest_done {
                Some(old) => {
                    self.agents.remove(&old);
                }
                None => return,
            }
        }
        self.agents.insert(
            id,
            TrackedAgent {
                agent_type,
                parent,
                status: SessionStatus::Running,
                started: now,
                last_event: now,
                tool_count: 0,
            },
        );
    }

    /// The agents whose parent is `parent`, each with their own subtree.
    fn agent_tree(&self, parent: Option<&str>, now: Instant, depth: usize) -> Vec<AgentNode> {
        if depth > 16 {
            return Vec::new();
        }
        let mut nodes: Vec<_> = self
            .agents
            .iter()
            .filter(|(id, agent)| agent.parent.as_deref() == parent && Some(id.as_str()) != parent)
            .map(|(id, agent)| {
                let until = if agent.status == SessionStatus::Done { agent.last_event } else { now };
                (
                    agent.started,
                    AgentNode {
                        agent_id: id.clone(),
                        agent_type: agent.agent_type.clone(),
                        status: agent.status,
                        elapsed_seconds: until.duration_since(agent.started).as_secs(),
                        tool_count: agent.tool_count,
                        children: self.agent_tree(Some(id), now, depth + 1),
                    },
                )
            })
            .collect();
        nodes.sort_by_key(|(started, node)| (*started, node.agent_id.clone()));
        nodes.into_iter().map(|(_, node)| node).collect()
    }
}

impl Sessions {
    pub(crate) fn with_threshold(seconds: Option<u64>) -> Self {
        let mut sessions = Self::default();
        if let Some(seconds) = seconds {
            sessions.stuck_threshold = Duration::from_secs(seconds);
        }
        sessions
    }

    pub(crate) fn record(
        &mut self,
        harness: &'static str,
        id: String,
        cwd: PathBuf,
        process_id: Option<u32>,
        actor: Actor,
        event: Event,
        now: Instant,
    ) {
        self.expire_ended(now);
        let key = (harness, id);
        if matches!(event, Event::Start) {
            self.by_id.insert(
                key,
                TrackedSession {
                    project_folder: cwd,
                    process_id,
                    status: SessionStatus::Done,
                    started: now,
                    last_event: now,
                    tool_count: 0,
                    agents: BTreeMap::new(),
                    pending_spawns: VecDeque::new(),
                },
            );
            return;
        }

        let Some(session) = self.by_id.get_mut(&key) else {
            return;
        };
        if session.status == SessionStatus::Ended {
            return;
        }
        session.project_folder = cwd;
        if process_id.is_some() { session.process_id = process_id; }
        session.last_event = now;
        match event {
            Event::SubagentStart => {
                if let Some(id) = actor.agent {
                    let parent = session.pending_spawns.pop_front().flatten().filter(|p| *p != id);
                    session.add_agent(id, actor.agent_type, parent, now);
                }
                return;
            }
            Event::SubagentStop => {
                if let Some(agent) = actor.agent.and_then(|id| session.agents.get_mut(&id)) {
                    agent.status = SessionStatus::Done;
                    agent.last_event = now;
                }
                return;
            }
            _ => {}
        }
        if let Some(id) = &actor.agent {
            // A subagent's own event; create its node if its start was never seen.
            if !session.agents.contains_key(id) {
                session.add_agent(id.clone(), actor.agent_type.clone(), None, now);
            }
            let agent = session.agents.get_mut(id).expect("just ensured");
            agent.last_event = now;
            match event {
                Event::ToolStarted => {
                    agent.tool_count += 1;
                    agent.status = SessionStatus::Running;
                }
                Event::Working => agent.status = SessionStatus::Running,
                Event::Permission => agent.status = SessionStatus::Blocked,
                _ => {}
            }
        }
        if actor.spawns_agent && matches!(event, Event::ToolStarted) {
            session.pending_spawns.push_back(actor.agent.clone());
            if session.pending_spawns.len() > MAX_PENDING_SPAWNS {
                session.pending_spawns.pop_front();
            }
        }
        if matches!(event, Event::Prompt) {
            session.agents.retain(|_, agent| agent.status != SessionStatus::Done);
            session.pending_spawns.clear();
        }
        // A subagent's Stop or Interrupt does not end the main agent's turn.
        if actor.agent.is_some() && matches!(event, Event::Stop) {
            return;
        }
        session.status = match event {
            Event::Start | Event::SubagentStart | Event::SubagentStop => {
                unreachable!("handled above")
            }
            Event::Working | Event::Prompt => SessionStatus::Running,
            Event::ToolStarted => {
                session.tool_count += 1;
                SessionStatus::Running
            }
            Event::Permission => SessionStatus::Blocked,
            Event::Stop => {
                for agent in session.agents.values_mut() {
                    if agent.status != SessionStatus::Done {
                        agent.status = SessionStatus::Done;
                    }
                }
                session.pending_spawns.clear();
                SessionStatus::Done
            }
            Event::End => SessionStatus::Ended,
        };
    }

    fn expire_ended(&mut self, now: Instant) {
        self.by_id.retain(|_, session| {
            session.status != SessionStatus::Ended
                || now.duration_since(session.last_event) < ENDED_RETENTION
        });
    }

    fn list(&mut self, now: Instant) -> Vec<Session> {
        self.expire_ended(now);
        let mut listed: Vec<_> = self
            .by_id
            .iter()
            .map(|((harness, id), session)| {
                let status = match session.status {
                    SessionStatus::Running
                        if now.duration_since(session.last_event) >= self.stuck_threshold =>
                    {
                        SessionStatus::Stalled
                    }
                    status => status,
                };
                let until = if status == SessionStatus::Ended {
                    session.last_event
                } else {
                    now
                };
                Session {
                    session_id: id.clone(),
                    harness: (*harness).into(),
                    project_folder: session.project_folder.clone(),
                    status,
                    elapsed_seconds: until.duration_since(session.started).as_secs(),
                    last_event_seconds: now.duration_since(session.last_event).as_secs(),
                    tool_count: session.tool_count,
                    agents: session.agent_tree(None, now, 0),
                }
            })
            .collect();
        listed.sort_by_key(|session| match session.status {
            SessionStatus::Blocked => 0,
            SessionStatus::Stalled => 1,
            SessionStatus::Running => 2,
            SessionStatus::Done => 3,
            SessionStatus::Ended => 4,
        });
        listed
    }

    pub(crate) fn target(&self, session_id: &str) -> Option<WindowTarget> {
        let mut matches = self.by_id.iter().filter(|((_, id), session)|
            id == session_id && session.status != SessionStatus::Ended);
        let (_, session) = matches.next()?;
        if matches.next().is_some() { return None; }
        Some(WindowTarget {
            process_id: session.process_id?,
            project_folder: session.project_folder.clone(),
        })
    }
}

impl State {
    pub(crate) fn list_sessions(&self) -> Vec<Session> {
        self.sessions.lock().unwrap().list(self.clock.now())
    }

    pub(crate) fn hooks_installed(&self) -> bool {
        let claude = self.home.join(".claude/settings.json");
        let codex_home = (std::env::home_dir().as_deref() == Some(self.home.as_path()))
            .then(|| std::env::var_os("CODEX_HOME"))
            .flatten()
            .filter(|path| !path.is_empty())
            .map(PathBuf::from)
            .unwrap_or_else(|| self.home.join(".codex"));
        [claude, codex_home.join("config.toml")].iter().any(|path| {
            std::fs::read_to_string(path).is_ok_and(|contents| contents.contains("skills-pad-"))
        })
    }
}

impl Core {
    /// Return current session snapshots, including recently ended sessions.
    pub fn list_sessions(&self) -> Vec<Session> {
        self.state.list_sessions()
    }

    /// Change the running-to-stalled timeout. It is saved in settings.
    pub fn set_stuck_threshold(&self, threshold: Duration) {
        self.state.sessions.lock().unwrap().stuck_threshold = threshold;
        let saved = self.state.change_settings(|s| {
            s.stuck_threshold_seconds = Some(threshold.as_secs());
            Ok(())
        });
        if let Err(error) = saved {
            eprintln!("agent-pad: {error}");
        }
    }

    /// The running-to-stalled timeout.
    pub fn stuck_threshold(&self) -> Duration {
        self.state.sessions.lock().unwrap().stuck_threshold
    }
}
