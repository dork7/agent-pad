//! Ticket 19: project tickets for a session over both HTTP APIs.

use std::fs;
use std::net::{Ipv4Addr, SocketAddr};
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};
use serde_json::json;
use agent_pad_core::testing::{FakeClock, RecordingPaster};
use agent_pad_core::{Core, CoreConfig, SettingsError, Ticket, TicketSummary};
use tempfile::TempDir;

const LIST: &str = "/sessions/work/tickets";

fn start(home: &TempDir) -> Core {
    Core::start(CoreConfig {
        home: home.path().to_path_buf(),
        paster: Arc::new(RecordingPaster::new()),
        clock: Arc::new(FakeClock::new()),
    })
    .unwrap()
}

fn start_session(core: &Core, id: &str, project: &Path) {
    let response = ureq::post(&core.url("/hooks/claude"))
        .timeout(Duration::from_secs(1))
        .send_json(json!({
            "session_id": id,
            "cwd": project,
            "hook_event_name": "SessionStart",
            "source": "startup",
        }))
        .unwrap();
    assert_eq!(response.status(), 204);
}

fn write(project: &Path, id: &str, markdown: &str) {
    let path = project.join(id);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, markdown).unwrap();
}

fn get(url: &str) -> ureq::Request {
    ureq::get(url).timeout(Duration::from_secs(1))
}

fn list(core: &Core) -> Vec<TicketSummary> {
    get(&core.url(LIST)).call().unwrap().into_json().unwrap()
}

fn ticket_path(list: &str, id: &str) -> String {
    format!("{list}/{}", utf8_percent_encode(id, NON_ALPHANUMERIC))
}

fn assert_status(request: ureq::Request, expected: u16) {
    let error = request.call().unwrap_err();
    match error {
        ureq::Error::Status(actual, _) => assert_eq!(actual, expected),
        _ => panic!("expected HTTP {expected}, got {error}"),
    }
}

#[test]
fn lists_pending_tickets_and_resolves_blockers_from_the_current_files() {
    let home = TempDir::new().unwrap();
    let project = TempDir::new().unwrap();
    let project = project.path();
    let core = start(&home);
    start_session(&core, "work", project);

    let followup = ".scratch/alpha/issues/20-followup + café # %.md";
    let markdown = "Introduction\n# 20: Follow-up\n# Another heading\n\n**Status:** Ready-For-Agent\n**Blocked by:** #14, ticket 2 (no GitHub issue yet), #91, #14, #999\n\n- [x] first\n  - [ ] nested\n- [ ] last\n";
    write(project, followup, markdown);
    let first = ".scratch/beta/issues/02-first.md";
    write(project, first, "# 02: First\n");
    let blocker = ".scratch/alpha/issues/13-blocker.md";
    write(project, blocker, "# 13: Blocker\n**GitHub:** #14\n");
    write(
        project,
        ".scratch/alpha/issues/14-closed.md",
        "# Closed\n**Status:** cLoSeD\n",
    );
    write(
        project,
        ".scratch/alpha/issues/03-done.md",
        "# Done\n**Status:** DONE\n**GitHub:** #91\n",
    );
    write(
        project,
        ".scratch/alpha/issues/04-manual.md",
        "# Manual checks\n**Status:** done (manual checks pending)\n",
    );
    write(
        project,
        ".scratch/alpha/issues/05-wontfix.md",
        "# Won't fix\n**Status:** WONTFIX\n",
    );

    let summaries = list(&core);
    assert_eq!(
        summaries
            .iter()
            .map(|ticket| ticket.number)
            .collect::<Vec<_>>(),
        [2, 13, 20]
    );
    assert_eq!(summaries, core.pending_tickets("work").unwrap());
    assert_eq!(summaries[0].status, None);
    assert!(!summaries[0].blocked);
    assert_eq!(summaries[0].blocked_by.raw, None);
    let summary = &summaries[2];
    assert_eq!(summary.id, followup);
    assert_eq!(summary.title, "Follow-up");
    assert_eq!(summary.status.as_deref(), Some("Ready-For-Agent"));
    assert!(summary.blocked);
    assert_eq!(summary.blocked_by.ticket_numbers, [13, 2, 3]);
    assert_eq!(
        summary.blocked_by.raw.as_deref(),
        Some("#14, ticket 2 (no GitHub issue yet), #91, #14, #999")
    );
    assert_eq!((summary.done, summary.total), (1, 3));

    let detail_url = core.url(&ticket_path(LIST, &summary.id));
    let detail: Ticket = get(&detail_url).call().unwrap().into_json().unwrap();
    assert_eq!(&detail.summary, summary);
    assert_eq!(detail.markdown, markdown);
    assert_eq!(detail, core.ticket("work", followup).unwrap());

    write(
        project,
        blocker,
        "# Blocker\n**GitHub:** #14\n**Status:** done\n",
    );
    assert!(list(&core).last().unwrap().blocked);
    write(project, first, "# First\n**Status:** closed\n");
    write(project, ".scratch/alpha/issues/10-new.md", "# New\n");
    let refreshed = list(&core);
    assert_eq!(
        refreshed
            .iter()
            .map(|ticket| ticket.number)
            .collect::<Vec<_>>(),
        [10, 20]
    );
    assert!(!refreshed[1].blocked);

    let edited = markdown.replace("Follow-up", "Updated title");
    write(project, followup, &edited);
    let detail: Ticket = get(&detail_url).call().unwrap().into_json().unwrap();
    assert_eq!(detail.markdown, edited);
    assert_eq!(detail.summary.title, "Updated title");
    assert!(!detail.summary.blocked);

    fs::remove_file(project.join(followup)).unwrap();
    assert_status(get(&detail_url), 404);
}

#[test]
fn scopes_ticket_ids_to_the_sessions_project_and_pattern() {
    let home = TempDir::new().unwrap();
    let temp = TempDir::new().unwrap();
    let project = temp.path().join("project");
    let other = temp.path().join("other");
    fs::create_dir(&project).unwrap();
    let core = start(&home);
    start_session(&core, "work", &project);
    assert!(list(&core).is_empty());
    start_session(&core, "missing-folder", &temp.path().join("missing"));
    let empty: Vec<TicketSummary> = get(&core.url("/sessions/missing-folder/tickets"))
        .call()
        .unwrap()
        .into_json()
        .unwrap();
    assert!(empty.is_empty());

    let foreign = ".scratch/other/issues/31-foreign.md";
    write(&other, foreign, "# Foreign\n");
    write(&project, "32-outside-pattern.md", "# Outside pattern\n");
    write(
        &project,
        ".scratch/work/issues/33-done.md",
        "# Done\n**Status:** closed\n",
    );
    start_session(&core, "other", &other);
    assert!(list(&core).is_empty());
    let other_list: Vec<TicketSummary> = get(&core.url("/sessions/other/tickets"))
        .call()
        .unwrap()
        .into_json()
        .unwrap();
    assert_eq!(other_list[0].id, foreign);

    let absolute = other.join(foreign);
    for id in [
        foreign,
        "32-outside-pattern.md",
        "../other/.scratch/other/issues/31-foreign.md",
        ".scratch/work/issues/../../../32-outside-pattern.md",
        "..\\other\\.scratch\\other\\issues\\31-foreign.md",
        absolute.to_str().unwrap(),
    ] {
        assert_status(get(&core.url(&ticket_path(LIST, id))), 404);
    }
    assert_status(get(&core.url(&format!("{LIST}/%FF"))), 404);
    assert_status(get(&core.url("/sessions/unknown/tickets")), 404);
    assert_status(
        get(&core.url(&ticket_path("/sessions/unknown/tickets", foreign))),
        404,
    );
}

#[cfg(unix)]
#[test]
fn rejects_ticket_file_and_directory_symlinks_escaping_the_project() {
    use std::os::unix::fs::symlink;

    let home = TempDir::new().unwrap();
    let temp = TempDir::new().unwrap();
    let project = temp.path().join("project");
    let other = temp.path().join("other");
    write(&project, ".scratch/work/issues/10-real.md", "# Real\n");
    write(&other, "11-private.md", "# Private\n");
    write(&other, "issues/12-private.md", "# Private\n");
    let file_link = ".scratch/work/issues/11-file.md";
    symlink(other.join("11-private.md"), project.join(file_link)).unwrap();
    symlink("../../other", project.join(".scratch/linked")).unwrap();

    let core = start(&home);
    start_session(&core, "work", &project);
    assert_eq!(
        list(&core)
            .iter()
            .map(|ticket| ticket.number)
            .collect::<Vec<_>>(),
        [10]
    );
    for id in [file_link, ".scratch/linked/issues/12-private.md"] {
        assert_status(get(&core.url(&ticket_path(LIST, id))), 404);
    }
}

#[test]
fn changes_the_pattern_on_the_next_request_and_persists_across_restarts() {
    let home = TempDir::new().unwrap();
    let project = TempDir::new().unwrap();
    let core = start(&home);
    let default_id = ".scratch/work/issues/01-default.md";
    let alternate_id = "tickets/work-active/02-alt.md";
    write(project.path(), default_id, "# Default\n");
    write(project.path(), alternate_id, "# Alternate\n");
    write(project.path(), "tickets/work-active/02-no.txt", "# No\n");
    write(
        project.path(),
        "tickets/work-active/nested/03-no.md",
        "# No\n",
    );
    start_session(&core, "work", project.path());

    assert_eq!(core.ticket_pattern(), None);
    assert_eq!(list(&core)[0].id, default_id);
    let pattern = "tickets/w*k*active/*.md".to_string();
    core.set_ticket_pattern(Some(pattern.clone())).unwrap();
    for invalid in [
        "../other/*.md",
        "tickets/../*.md",
        "/tmp/*.md",
        "C:/tmp/*.md",
        "",
    ] {
        assert_eq!(
            core.set_ticket_pattern(Some(invalid.to_string())),
            Err(SettingsError::InvalidTicketPattern)
        );
    }
    assert_status(get(&core.url(&ticket_path(LIST, default_id))), 404);
    let summaries = list(&core);
    assert_eq!(summaries.len(), 1);
    assert_eq!(summaries[0].id, alternate_id);

    drop(core);
    let core = start(&home);
    start_session(&core, "work", project.path());
    assert_eq!(core.ticket_pattern().as_deref(), Some(pattern.as_str()));
    assert_eq!(list(&core), summaries);
    core.set_ticket_pattern(None).unwrap();
    assert_eq!(list(&core)[0].id, default_id);
    assert_status(get(&core.url(&ticket_path(LIST, alternate_id))), 404);
}

#[test]
fn paired_phones_can_list_and_fetch_tickets() {
    let home = TempDir::new().unwrap();
    let project = TempDir::new().unwrap();
    let id = ".scratch/work/issues/01-phone.md";
    write(project.path(), id, "# Phone\n");
    let core = start(&home);
    start_session(&core, "work + phone", project.path());
    core.set_sharing_address_override(Some(Ipv4Addr::LOCALHOST.into()));
    let sharing = core.start_sharing().unwrap();
    let base = format!("http://{}", SocketAddr::new(sharing.address, sharing.port));
    let list_url = format!("{base}/sessions/work%20%2B%20phone/tickets");
    let detail_url = ticket_path(&list_url, id);

    assert_status(get(&list_url), 401);
    assert_status(get(&detail_url), 401);
    let paired = get(&sharing.url).call().unwrap();
    let cookie = paired
        .all("Set-Cookie")
        .into_iter()
        .map(|value| value.split(';').next().unwrap())
        .collect::<Vec<_>>()
        .join("; ");

    let summaries: Vec<TicketSummary> = get(&list_url)
        .set("Cookie", &cookie)
        .call()
        .unwrap()
        .into_json()
        .unwrap();
    assert_eq!(summaries, core.pending_tickets("work + phone").unwrap());
    let detail: Ticket = get(&detail_url)
        .set("Cookie", &cookie)
        .call()
        .unwrap()
        .into_json()
        .unwrap();
    assert_eq!(detail.summary, summaries[0]);
    assert_eq!(detail.markdown, "# Phone\n");

    assert_status(
        get(&format!("{base}/sessions/unknown/tickets")).set("Cookie", &cookie),
        404,
    );
    assert_status(
        get(&ticket_path(&list_url, "../01-phone.md")).set("Cookie", &cookie),
        404,
    );
}
