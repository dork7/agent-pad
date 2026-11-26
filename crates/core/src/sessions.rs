//! In-memory session state. Ended sessions retain their final elapsed time and
//! disappear after ten minutes. A Core restart starts with an empty session list.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

use crate::{Core, State};

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
}

pub(crate) enum Event {
    Start,
    Working,
    ToolStarted,
    Permission,
    Stop,
    End,
}

struct TrackedSession {
    project_folder: PathBuf,
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
                    status: SessionStatus::Done,
                    started: now,
                    last_event: now,
                    tool_count: 0,
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
        session.last_event = now;
        session.status = match event {
            Event::Start => unreachable!("start handled above"),
            Event::Working => SessionStatus::Running,
            Event::ToolStarted => {
                session.tool_count += 1;
                SessionStatus::Running
            }
            Event::Permission => SessionStatus::Blocked,
            Event::Stop => SessionStatus::Done,
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
