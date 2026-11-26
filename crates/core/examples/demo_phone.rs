//! A safe demo of the phone page with sample data: `cargo run -p agent-pad-core --example demo_phone`.
//!
//! Uses a temporary home with a few skills, a preset, snippets, a project with tickets and a fake
//! Claude Code session. Taps are recorded and printed, never pasted, so it is safe for
//! screenshots and trying the UI.

use std::fs;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use agent_pad_core::testing::RecordingPaster;
use agent_pad_core::{Core, CoreConfig, SystemClock};

fn skill(dir: &Path, name: &str, description: &str) {
    let dir = dir.join(name);
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("SKILL.md"),
        format!("---\nname: {name}\ndescription: {description}\n---\n"),
    )
    .unwrap();
}

fn main() -> std::io::Result<()> {
    let home = tempfile::TempDir::new()?;
    let claude = home.path().join(".claude/skills");
    skill(
        &claude,
        "code-review",
        "Review the changes since a fixed point against standards and spec.",
    );
    skill(
        &claude,
        "tdd",
        "Test-driven development with red-green-refactor.",
    );
    skill(
        &claude,
        "diagnosing-bugs",
        "Diagnosis loop for hard bugs and performance regressions.",
    );
    skill(
        &home.path().join(".codex/skills"),
        "plan",
        "Write an implementation plan.",
    );

    let project = home.path().join("work/app");
    let issues = project.join(".scratch/app/issues");
    fs::create_dir_all(&issues)?;
    fs::write(
        issues.join("01-login.md"),
        "# 01: Login page\n\n**Status:** done\n\n**GitHub:** #2\n",
    )?;
    fs::write(
        issues.join("02-search.md"),
        "# 02: Search\n\n**Status:** ready-for-agent\n\n**GitHub:** #3\n\n- [x] Index\n- [ ] Query box\n- [ ] Results\n",
    )?;
    fs::write(
        issues.join("03-filters.md"),
        "# 03: Filters\n\n**Blocked by:** #3\n\n**Status:** triage\n\n- [ ] Facets\n",
    )?;

    let paster = Arc::new(RecordingPaster::new());
    let core = Core::start(CoreConfig {
        home: home.path().to_path_buf(),
        paster: paster.clone(),
        clock: Arc::new(SystemClock),
    })?;
    if let Some(address) = std::env::args().nth(1) {
        core.set_sharing_address_override(Some(address.parse().expect("an IP address")));
    }
    core.add_preset("claude:code-review", "high").unwrap();
    core.add_preset("claude:code-review", "focus on tests")
        .unwrap();
    core.add_snippet("Commit message", "feat: \n\nCo-Authored-By: Claude")
        .unwrap();
    core.add_snippet("Repo path", "~/work/app").unwrap();
    ureq_post(&core.url("/hooks/claude"), &project, "SessionStart");
    ureq_post(&core.url("/hooks/claude"), &project, "UserPromptSubmit");

    let info = core.start_sharing()?;
    println!("Open on your phone: {}", info.url);
    let mut seen = 0;
    loop {
        std::thread::sleep(Duration::from_millis(500));
        let pasted = paster.pasted();
        for text in &pasted[seen..] {
            println!("(would paste) {text:?}");
        }
        seen = pasted.len();
    }
}

/// Sends a fake Claude Code hook event, as the installed hook would.
fn ureq_post(url: &str, project: &Path, event: &str) {
    let body = format!(
        r#"{{"session_id":"demo","cwd":{:?},"hook_event_name":"{event}","source":"startup"}}"#,
        project.to_string_lossy()
    );
    let addr = url.trim_start_matches("http://");
    let (host, path) = addr.split_once('/').unwrap();
    use std::io::{Read, Write};
    let mut stream = std::net::TcpStream::connect(host).unwrap();
    write!(
        stream,
        "POST /{path} HTTP/1.1\r\nHost: {host}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    )
    .unwrap();
    let mut response = String::new();
    let _ = stream.read_to_string(&mut response);
}
