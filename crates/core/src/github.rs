//! Open GitHub issues for a project folder, fetched with the `gh` CLI the user already has.
//!
//! Failures (no `gh`, not a repo, not logged in, offline, slow) are never errors: the Session
//! screen then shows only the local Markdown tickets.

use std::collections::HashMap;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use serde::Deserialize;

const CACHE_FOR: Duration = Duration::from_secs(30);
const GH_TIMEOUT: Duration = Duration::from_secs(8);
const MAX_ISSUES: &str = "100";

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct GithubIssue {
    pub number: u32,
    pub title: String,
    #[serde(default)]
    pub body: String,
}

/// Where open issues come from. Replaced with a fake in tests.
pub trait GithubIssues: Send + Sync {
    fn open_issues(&self, project: &Path) -> Vec<GithubIssue>;
}

/// Runs `gh issue list` in the project folder, caching each project's answer briefly so
/// opening and re-opening the ticket list does not hit GitHub every time.
#[derive(Default)]
pub struct SystemGithubIssues {
    cache: Mutex<HashMap<PathBuf, (Instant, Vec<GithubIssue>)>>,
}

impl GithubIssues for SystemGithubIssues {
    fn open_issues(&self, project: &Path) -> Vec<GithubIssue> {
        if !inside_git_repo(project) {
            return Vec::new();
        }
        if let Some((at, issues)) = self.cache.lock().unwrap().get(project) {
            if at.elapsed() < CACHE_FOR {
                return issues.clone();
            }
        }
        let issues = run_gh(project).unwrap_or_default();
        self.cache
            .lock()
            .unwrap()
            .insert(project.to_path_buf(), (Instant::now(), issues.clone()));
        issues
    }
}

fn inside_git_repo(project: &Path) -> bool {
    project.ancestors().any(|dir| dir.join(".git").exists())
}

fn run_gh(project: &Path) -> Option<Vec<GithubIssue>> {
    let mut child = Command::new("gh")
        .args(["issue", "list", "--state", "open", "--limit", MAX_ISSUES])
        .args(["--json", "number,title,body"])
        .current_dir(project)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;
    let mut stdout = child.stdout.take()?;
    let reader = std::thread::spawn(move || {
        let mut text = String::new();
        stdout.read_to_string(&mut text).ok().map(|_| text)
    });
    let started = Instant::now();
    let status = loop {
        match child.try_wait().ok()? {
            Some(status) => break status,
            None if started.elapsed() > GH_TIMEOUT => {
                let _ = child.kill();
                let _ = child.wait();
                return None;
            }
            None => std::thread::sleep(Duration::from_millis(25)),
        }
    };
    let text = reader.join().ok()??;
    status
        .success()
        .then(|| serde_json::from_str(&text).ok())
        .flatten()
}
