//! Tickets 08 and 18: what the embedded phone page promises. The real wake lock, haptics and
//! one-handed feel are manual checks on real phones; these tests pin the page's contract.

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
