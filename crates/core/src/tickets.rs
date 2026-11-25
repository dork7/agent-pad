//! Read tickets from the project folder reported by a session's harness.

use std::collections::{HashMap, HashSet};
use std::io;
use std::path::Path;

use cap_std::{ambient_authority, fs::Dir};
use serde::{Deserialize, Serialize};

use crate::{Core, SettingsError, State};

pub const DEFAULT_TICKET_PATTERN: &str = ".scratch/*/issues/*.md";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlockedBy {
    /// The value of the Markdown metadata line, if present.
    pub raw: Option<String>,
    /// Deduplicated local ticket numbers in the order they were referenced.
    pub ticket_numbers: Vec<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TicketSummary {
    /// Slash-separated path relative to the session's project folder.
    pub id: String,
    pub number: u32,
    pub title: String,
    pub status: Option<String>,
    pub blocked: bool,
    pub blocked_by: BlockedBy,
    pub done: usize,
    pub total: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Ticket {
    #[serde(flatten)]
    pub summary: TicketSummary,
    pub markdown: String,
}

#[derive(Debug)]
pub enum TicketError {
    UnknownSession,
    UnknownTicket,
    InvalidPattern,
    Io(io::Error),
}

impl std::fmt::Display for TicketError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownSession => f.write_str("unknown session"),
            Self::UnknownTicket => f.write_str("unknown ticket"),
            Self::InvalidPattern => f.write_str("invalid ticket pattern"),
            Self::Io(error) => write!(f, "could not open ticket project: {error}"),
        }
    }
}

impl std::error::Error for TicketError {}

impl Core {
    /// List pending tickets from the session's project, rereading the files each time.
    pub fn pending_tickets(&self, session_id: &str) -> Result<Vec<TicketSummary>, TicketError> {
        self.state.pending_tickets(session_id)
    }

    /// Resolve a ticket ID against the session's current project and pattern.
    pub fn ticket(&self, session_id: &str, ticket_id: &str) -> Result<Ticket, TicketError> {
        self.state.ticket(session_id, ticket_id)
    }

    /// Save a project-relative pattern, or pass None to restore the default.
    pub fn set_ticket_pattern(&self, pattern: Option<String>) -> Result<(), SettingsError> {
        if pattern
            .as_deref()
            .is_some_and(|value| !relative_path(value))
        {
            return Err(SettingsError::InvalidTicketPattern);
        }
        self.state.change_settings(|settings| {
            settings.ticket_pattern = pattern;
            Ok(())
        })
    }

    /// The saved override, if any.
    pub fn ticket_pattern(&self) -> Option<String> {
        self.state.settings.lock().unwrap().ticket_pattern.clone()
    }
}

impl State {
    pub(crate) fn pending_tickets(
        &self,
        session_id: &str,
    ) -> Result<Vec<TicketSummary>, TicketError> {
        Ok(self
            .project_tickets(session_id)?
            .into_iter()
            .map(|ticket| ticket.summary)
            .filter(TicketSummary::is_pending)
            .collect())
    }

    pub(crate) fn ticket(&self, session_id: &str, ticket_id: &str) -> Result<Ticket, TicketError> {
        if !relative_path(ticket_id) {
            return Err(TicketError::UnknownTicket);
        }
        self.project_tickets(session_id)?
            .into_iter()
            .find(|ticket| ticket.summary.id == ticket_id)
            .ok_or(TicketError::UnknownTicket)
    }

    fn project_tickets(&self, session_id: &str) -> Result<Vec<Ticket>, TicketError> {
        let session = self
            .list_sessions()
            .into_iter()
            .find(|session| session.session_id == session_id)
            .ok_or(TicketError::UnknownSession)?;
        let pattern = self.settings.lock().unwrap().ticket_pattern.clone();
        let pattern = pattern.as_deref().unwrap_or(DEFAULT_TICKET_PATTERN);
        if !relative_path(pattern) {
            return Err(TicketError::InvalidPattern);
        }
        if !session.project_folder.is_absolute() {
            return Ok(Vec::new());
        }
        // Confine all descendant opens to the project handle, including symlink resolution.
        let root = match Dir::open_ambient_dir(session.project_folder, ambient_authority()) {
            Ok(root) => root,
            Err(error)
                if matches!(
                    error.kind(),
                    io::ErrorKind::NotFound | io::ErrorKind::NotADirectory
                ) =>
            {
                return Ok(Vec::new());
            }
            Err(error) => return Err(TicketError::Io(error)),
        };

        let mut tickets = Vec::new();
        for id in matching_paths(&root, pattern) {
            let Some(number) = ticket_number(&id) else {
                continue;
            };
            if root.is_file(&id) {
                if let Ok(markdown) = root.read_to_string(&id) {
                    tickets.push(parse(id, number, markdown));
                }
            }
        }
        tickets.sort_by(|(left, _), (right, _)| {
            left.summary
                .number
                .cmp(&right.summary.number)
                .then_with(|| left.summary.id.cmp(&right.summary.id))
        });

        let mut github = HashMap::new();
        let mut local = HashSet::new();
        let mut pending = HashSet::new();
        for (ticket, issue) in &tickets {
            let summary = &ticket.summary;
            local.insert(summary.number);
            if let Some(issue) = issue {
                github.entry(*issue).or_insert(summary.number);
            }
            if summary.is_pending() {
                pending.insert(summary.number);
            }
        }
        Ok(tickets
            .into_iter()
            .map(|(mut ticket, _)| {
                let summary = &mut ticket.summary;
                summary.blocked_by.ticket_numbers =
                    blocker_numbers(summary.blocked_by.raw.as_deref(), &github, &local);
                summary.blocked = summary
                    .blocked_by
                    .ticket_numbers
                    .iter()
                    .any(|number| pending.contains(number));
                ticket
            })
            .collect())
    }
}

impl TicketSummary {
    fn is_pending(&self) -> bool {
        self.status.as_ref().is_none_or(|value| {
            let status = value.to_ascii_lowercase();
            !status.starts_with("done") && status != "closed" && status != "wontfix"
        })
    }
}

/// IDs and patterns use forward slashes on every platform. Reject Windows prefixes
/// and alternate path separators even on Unix so saved patterns are portable.
fn relative_path(path: &str) -> bool {
    !path.contains(['\\', ':', '\0']) && path.split('/').all(normal_segment)
}

fn normal_segment(segment: &str) -> bool {
    !segment.is_empty() && segment != "." && segment != ".."
}

fn matching_paths(root: &Dir, pattern: &str) -> Vec<String> {
    let mut paths = vec![String::new()];
    for segment in pattern.split('/') {
        paths = paths
            .into_iter()
            .flat_map(|parent| {
                let directory = if parent.is_empty() { "." } else { &parent };
                root.read_dir(directory)
                    .into_iter()
                    .flatten()
                    .filter_map(|entry| entry.ok()?.file_name().into_string().ok())
                    .filter(|name| relative_path(name) && segment_matches(segment, name))
                    .map(move |name| {
                        if parent.is_empty() {
                            name
                        } else {
                            format!("{parent}/{name}")
                        }
                    })
            })
            .collect();
    }
    paths
}

fn segment_matches(pattern: &str, name: &str) -> bool {
    let Some((prefix, rest)) = pattern.split_once('*') else {
        return pattern == name;
    };
    let mut parts = rest.split('*');
    let suffix = parts.next_back().expect("split has at least one segment");
    let Some(mut middle) = name
        .strip_prefix(prefix)
        .and_then(|name| name.strip_suffix(suffix))
    else {
        return false;
    };
    for part in parts {
        let Some((_, rest)) = middle.split_once(part) else {
            return false;
        };
        middle = rest;
    }
    true
}

fn ticket_number(id: &str) -> Option<u32> {
    let path = Path::new(id);
    if !path.extension()?.eq_ignore_ascii_case("md") {
        return None;
    }
    let filename = path.file_name()?.to_str()?;
    number_prefix(filename)
}

fn number_prefix(text: &str) -> Option<u32> {
    let end = text
        .find(|ch: char| !ch.is_ascii_digit())
        .unwrap_or(text.len());
    text[..end].parse().ok()
}

fn parse(id: String, number: u32, markdown: String) -> (Ticket, Option<u32>) {
    let value = |label: &str| {
        markdown
            .lines()
            .find_map(|line| line.strip_prefix(label))
            .map(str::trim)
    };
    let title = markdown
        .lines()
        .find_map(|line| line.strip_prefix("# "))
        .map(|heading| {
            let heading = heading.trim();
            heading
                .split_once(':')
                .filter(|(prefix, _)| prefix.parse::<u32>().is_ok())
                .map_or(heading, |(_, title)| title.trim_start())
        })
        .unwrap_or_else(|| {
            Path::new(&id)
                .file_stem()
                .and_then(|stem| stem.to_str())
                .unwrap_or(&id)
        })
        .to_owned();
    let status = value("**Status:**").map(str::to_owned);
    let raw = value("**Blocked by:**").map(str::to_owned);
    let github = value("**GitHub:**")
        .and_then(|value| value.strip_prefix('#'))
        .and_then(number_prefix);
    let mut done = 0;
    let mut total = 0;
    for line in markdown.lines().map(str::trim_start) {
        if line.starts_with("- [x]") || line.starts_with("- [X]") {
            done += 1;
            total += 1;
        } else if line.starts_with("- [ ]") {
            total += 1;
        }
    }
    let summary = TicketSummary {
        id,
        number,
        title,
        status,
        blocked: false,
        blocked_by: BlockedBy {
            raw,
            ticket_numbers: Vec::new(),
        },
        done,
        total,
    };
    (Ticket { summary, markdown }, github)
}

fn blocker_numbers(
    raw: Option<&str>,
    github: &HashMap<u32, u32>,
    local: &HashSet<u32>,
) -> Vec<u32> {
    let mut numbers = Vec::new();
    let mut previous = "";
    for word in raw
        .unwrap_or_default()
        .split(|ch: char| !ch.is_ascii_alphanumeric() && ch != '#')
        .filter(|word| !word.is_empty())
    {
        let number = if let Some(issue) = word.strip_prefix('#').and_then(|word| word.parse().ok())
        {
            github
                .get(&issue)
                .copied()
                .or_else(|| local.contains(&issue).then_some(issue))
        } else if previous.eq_ignore_ascii_case("ticket") {
            word.parse().ok()
        } else {
            None
        };
        if let Some(number) = number {
            if !numbers.contains(&number) {
                numbers.push(number);
            }
        }
        previous = word;
    }
    numbers
}

/// Placeholder for the Ticket's path in an implement template.
pub const TICKET_PATH: &str = "{path}";

/// Default implement template for every Harness.
pub const DEFAULT_IMPLEMENT_TEMPLATE: &str = "Implement the ticket at {path}";

/// Why an "Implement" request was refused.
#[derive(Debug)]
pub enum ImplementError {
    Ticket(TicketError),
    /// The Ticket is done.
    NotPending,
    /// A Ticket it is blocked by is still pending.
    Blocked,
}

impl From<TicketError> for ImplementError {
    fn from(error: TicketError) -> Self {
        Self::Ticket(error)
    }
}

impl State {
    /// The implement template for `harness`.
    pub(crate) fn implement_template(&self, harness: &str) -> String {
        self.settings
            .lock()
            .unwrap()
            .implement_templates
            .get(harness)
            .cloned()
            .unwrap_or_else(|| DEFAULT_IMPLEMENT_TEMPLATE.into())
    }

    /// Pastes the implement text for a pending, unblocked Ticket of a Session's project, then
    /// presses Enter so the agent starts. Returns the pasted text.
    pub(crate) fn implement_ticket(
        &self,
        session_id: &str,
        ticket_id: &str,
    ) -> Result<String, ImplementError> {
        let ticket = self.ticket(session_id, ticket_id)?;
        if !ticket.summary.is_pending() {
            return Err(ImplementError::NotPending);
        }
        if ticket.summary.blocked {
            return Err(ImplementError::Blocked);
        }
        let harness = self
            .list_sessions()
            .into_iter()
            .find(|s| s.session_id == session_id)
            .map(|s| s.harness)
            .ok_or(ImplementError::Ticket(TicketError::UnknownSession))?;
        let text = self
            .implement_template(&harness)
            .replace(TICKET_PATH, ticket_id)
            .trim_end()
            .to_string();
        self.pastes.paste(text.clone(), true);
        Ok(text)
    }
}

impl Core {
    /// Pastes "implement this Ticket" into the focused app and presses Enter (ticket 20).
    pub fn implement_ticket(
        &self,
        session_id: &str,
        ticket_id: &str,
    ) -> Result<String, ImplementError> {
        self.state.implement_ticket(session_id, ticket_id)
    }

    /// The implement template for `harness`; `{path}` is the Ticket's project-relative path.
    pub fn implement_template(&self, harness: &str) -> String {
        self.state.implement_template(harness)
    }

    /// Sets (or with `None`, resets) the implement template for `harness`.
    pub fn set_implement_template(
        &self,
        harness: &str,
        template: Option<&str>,
    ) -> Result<(), SettingsError> {
        if template.is_some_and(|t| !t.contains(TICKET_PATH)) {
            return Err(SettingsError::TemplateWithoutPath);
        }
        self.state.change_settings(|s| {
            match template {
                Some(t) => s
                    .implement_templates
                    .insert(harness.to_string(), t.to_string()),
                None => s.implement_templates.remove(harness),
            };
            Ok(())
        })
    }
}
