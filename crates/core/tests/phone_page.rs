//! Contracts for the embedded phone page served by Core. Browser behavior, including the
//! wake lock, haptics and one-handed layout, is checked separately.

use std::net::{Ipv4Addr, SocketAddr};
use std::sync::Arc;

use skills_pad_core::testing::{FakeClock, RecordingPaster};
use skills_pad_core::{Core, CoreConfig};
use tempfile::TempDir;

fn phone_page() -> String {
    let home = TempDir::new().unwrap();
    let core = Core::start(CoreConfig {
        home: home.path().to_path_buf(),
        paster: Arc::new(RecordingPaster::new()),
        clock: Arc::new(FakeClock::new()),
    })
    .unwrap();
    core.set_sharing_address_override(Some(Ipv4Addr::LOCALHOST.into()));
    let info = core.start_sharing().unwrap();
    assert!(info.url.starts_with(&format!(
        "http://{}",
        SocketAddr::new(info.address, info.port)
    )));
    ureq::get(&info.url).call().unwrap().into_string().unwrap()
}

#[test]
fn the_page_heartbeats_every_30_seconds_while_visible() {
    let page = phone_page();
    assert!(page.contains("const HEARTBEAT_MS = 30 * 1000;"));
    assert!(page.contains("fetch('/heartbeat', { method: 'POST' })"));
    assert!(page.contains("if (!document.hidden)"));
}

#[test]
fn the_page_keeps_the_screen_awake_with_a_fallback_for_plain_http() {
    let page = phone_page();
    assert!(page.contains("navigator.wakeLock.request('screen')"));
    assert!(page.contains("window.isSecureContext"));
    // The fallback video is inline (iOS needs range requests for served video).
    assert!(page.contains("data:video/mp4;base64,AAAA"));
    // Released when hidden or left, re-acquired on return.
    assert!(page.contains("if (document.hidden) { releaseAwake(); return; }"));
    assert!(page.contains("addEventListener('pagehide', releaseAwake)"));
}

#[test]
fn the_page_follows_the_phone_theme_and_puts_controls_in_thumb_reach() {
    let page = phone_page();
    assert!(page.contains("@media (prefers-color-scheme: dark)"));
    assert!(page.contains("navigator.vibrate"));
    // Tabs and search live in the bottom dock, after the list.
    let list = page.find("<div id=\"list\"").unwrap();
    let dock = page.find("<div class=\"dock\">").unwrap();
    let search = page.find("<input id=\"q\"").unwrap();
    assert!(list < dock && dock < search);
    assert!(page.contains("width=device-width"));
}

#[test]
fn the_agent_sheet_polls_sessions_while_visible_and_supports_back_navigation() {
    let page = phone_page();
    assert!(page.contains("const SESSION_POLL_MS = 3000;"));
    assert!(page.contains("setInterval(pollSessions, SESSION_POLL_MS)"));
    assert!(page.contains("if (document.hidden || pollingSessions) return;"));
    assert!(page.contains("fetch('/sessions', { cache: 'no-store' })"));
    assert!(page.contains("aria-expanded=\"false\" aria-controls=\"sessions\""));
    assert!(page.contains(
        "No agent sessions. Install hooks in the Pad to see Claude Code and Codex sessions."
    ));
    for status in ["'Working'", "'Needs input'", "'Idle'", "'Stuck'", "'Ended'"] {
        assert!(page.contains(status), "missing session label {status}");
    }
    assert!(page.contains("history.pushState(") && page.contains("addEventListener('popstate'"));
    assert!(page.contains("$('back').addEventListener('click', () => history.back())"));
}

#[test]
fn ticket_reads_use_encoded_ids_and_render_read_only_content() {
    let page = phone_page();
    assert!(page.contains("`/sessions/${encodeURIComponent(sid)}/tickets`"));
    assert!(page.contains("`${list}/${encodeURIComponent(tid)}`"));
    assert!(page.contains("fetch(ticketsPath(current.sid, current.tid), { cache: 'no-store' })"));
    assert!(page.contains("'No pending tickets in this project.'"));
    for status in ["triage", "ready-for-agent", "in-progress", "blocked"] {
        assert!(page.contains(&format!(".ticket-status[data-status=\"{status}\"]")));
    }
    assert!(page.contains("textContent: ticket.status"));
    assert!(page.contains("renderMarkdown(ticket.markdown)"));
    assert!(page.contains("type: 'checkbox', disabled: true"));
    for html_sink in [
        "innerHTML",
        "outerHTML",
        "insertAdjacentHTML",
        "document.write",
    ] {
        assert!(
            !page.contains(html_sink),
            "unexpected HTML sink {html_sink}"
        );
    }
}

#[test]
fn implement_posts_only_ids_after_the_focus_note_and_reports_outcomes() {
    let page = phone_page();
    assert!(page.contains("fetch(`${ticketsPath(sid, ticket.id)}/implement`, { method: 'POST' })"));
    assert!(page.contains("disabled: ticket.blocked"));
    assert!(page.contains("`Blocked by ${blockers}`"));
    assert!(page.contains("The text is pasted into whichever app has focus on the computer — focus that session's terminal first."));
    assert!(page.contains("if (!implementConfirmed)"));
    assert!(page.contains("typeof body?.error === 'string'"));
    assert!(page.contains("toast('Sent to agent')"));
}
