//! Ticket 03: pair a Phone and control sharing through the Core.

use std::fs;
use std::net::{IpAddr, Ipv4Addr, SocketAddr, TcpStream, UdpSocket};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use serde_json::Value;
use agent_pad_core::testing::{FakeClock, RecordingPaster};
use agent_pad_core::{Core, CoreConfig, SharingInfo};
use tempfile::TempDir;

fn start(home: &TempDir, clock: Arc<FakeClock>) -> Core {
    Core::start(CoreConfig {
        home: home.path().to_path_buf(),
        paster: Arc::new(RecordingPaster::new()),
        clock,
    })
    .unwrap()
}

fn loopback_core(home: &TempDir, clock: Arc<FakeClock>) -> Core {
    let core = start(home, clock);
    core.set_sharing_address_override(Some(Ipv4Addr::LOCALHOST.into()));
    core
}

fn url(info: &SharingInfo, path: &str) -> String {
    format!("http://{}{path}", SocketAddr::new(info.address, info.port))
}

fn get(url: &str) -> ureq::Request {
    ureq::get(url).timeout(Duration::from_secs(1))
}

fn post(url: &str) -> ureq::Request {
    ureq::post(url).timeout(Duration::from_secs(1))
}

fn pairing_cookie(info: &SharingInfo) -> String {
    let response = get(&info.url).call().unwrap();
    assert_eq!(response.status(), 200);
    assert_eq!(
        response.header("Content-Type"),
        Some("text/html; charset=utf-8")
    );
    let set_cookie = response
        .all("Set-Cookie")
        .into_iter()
        .find(|c| c.starts_with("skills_pad_token="))
        .unwrap()
        .to_string();
    assert!(set_cookie.contains("Path=/"));
    assert!(set_cookie.contains("HttpOnly"));
    assert!(response
        .into_string()
        .unwrap()
        .contains("<h1>Agent Pad</h1>"));
    set_cookie.split(';').next().unwrap().to_string()
}

fn assert_unauthorized(result: Result<ureq::Response, ureq::Error>) {
    match result {
        Err(ureq::Error::Status(401, _)) => {}
        other => panic!("expected 401, got {other:?}"),
    }
}

fn assert_offline(info: &SharingInfo) {
    let result = get(&info.url).call();
    assert!(
        matches!(result, Err(ureq::Error::Transport(_))),
        "sharing still responds over HTTP: {result:?}"
    );
}

fn wait_for_idle_stop(core: &Core, sharing: &SharingInfo) {
    // TCP probes do not send HTTP requests, so only the background idle check
    // can stop sharing here.
    let addr = SocketAddr::new(sharing.address, sharing.port);
    let deadline = Instant::now() + Duration::from_secs(3);
    while TcpStream::connect_timeout(&addr, Duration::from_millis(100)).is_ok() {
        assert!(Instant::now() < deadline, "sharing did not stop when idle");
        thread::sleep(Duration::from_millis(50));
    }
    while core.sharing_status().is_some() {
        assert!(Instant::now() < deadline, "sharing status stayed active");
        thread::sleep(Duration::from_millis(10));
    }
    assert_offline(sharing);
}

#[test]
fn sharing_starts_off_and_uses_a_separate_address_and_port() {
    let home = TempDir::new().unwrap();
    let core = start(&home, Arc::new(FakeClock::new()));

    // A host without an outbound IPv4 route must supply an override.
    let socket = UdpSocket::bind("0.0.0.0:0").unwrap();
    if socket.connect("192.0.2.1:80").is_ok() {
        assert_eq!(
            core.sharing_address().unwrap(),
            socket.local_addr().unwrap().ip()
        );
    } else {
        assert!(core.sharing_address().is_err());
    }
    assert!(core.sharing_status().is_none());
    assert_eq!(core.addr().ip(), IpAddr::V4(Ipv4Addr::LOCALHOST));
    assert!(matches!(
        get(&core.url("/")).call(),
        Err(ureq::Error::Status(404, _))
    ));

    core.set_sharing_address_override(Some(Ipv4Addr::LOCALHOST.into()));
    assert_eq!(core.sharing_address().unwrap(), Ipv4Addr::LOCALHOST);
    assert!(core.sharing_status().is_none());

    let sharing = core.start_sharing().unwrap();
    assert_eq!(core.sharing_status().as_ref(), Some(&sharing));
    assert_eq!(sharing.address, Ipv4Addr::LOCALHOST);
    assert_ne!(sharing.port, 0);
    assert_ne!(sharing.port, core.addr().port());
    let token = sharing
        .url
        .strip_prefix(&url(&sharing, "/?token="))
        .unwrap();
    assert_eq!(token.len(), 32);
    assert!(token.bytes().all(|byte| byte.is_ascii_hexdigit()));
    assert!(sharing.qr_svg.contains("<svg "));
    assert!(sharing.qr_svg.contains("</svg>"));
    pairing_cookie(&sharing);
}

#[test]
fn phone_endpoints_require_the_token_or_cookie_and_share_the_local_skill_list() {
    let home = TempDir::new().unwrap();
    let core = loopback_core(&home, Arc::new(FakeClock::new()));
    let sharing = core.start_sharing().unwrap();
    let page = url(&sharing, "/");
    let skills = url(&sharing, "/skills");
    let heartbeat = url(&sharing, "/heartbeat");

    assert_unauthorized(get(&page).call());
    assert_unauthorized(get(&format!("{page}?token=invalid")).call());
    assert_unauthorized(get(&skills).call());
    assert_unauthorized(post(&heartbeat).call());

    let cookie = pairing_cookie(&sharing);
    let phone_skills = || -> Vec<Value> {
        get(&skills)
            .set("Cookie", &cookie)
            .call()
            .unwrap()
            .into_json()
            .unwrap()
    };
    assert!(phone_skills().is_empty());

    let dir = home.path().join(".claude/skills/tdd");
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("SKILL.md"), "---\nname: tdd\n---\n").unwrap();

    let local_skills: Vec<Value> = get(&core.url("/skills"))
        .call()
        .unwrap()
        .into_json()
        .unwrap();
    assert_eq!(local_skills[0]["id"], "claude:tdd");
    assert_eq!(phone_skills(), local_skills);
    assert_eq!(
        get(&page).set("Cookie", &cookie).call().unwrap().status(),
        200
    );
    assert_eq!(
        post(&heartbeat)
            .set("Cookie", &cookie)
            .call()
            .unwrap()
            .status(),
        204
    );
}

#[test]
fn stopping_closes_sharing_immediately_and_restarting_rejects_previous_credentials() {
    let home = TempDir::new().unwrap();
    let core = loopback_core(&home, Arc::new(FakeClock::new()));
    let first = core.start_sharing().unwrap();
    let old_token = first.url.split_once("?token=").unwrap().1;
    let old_cookie = pairing_cookie(&first);

    core.stop_sharing();
    assert!(core.sharing_status().is_none());
    assert_offline(&first);
    assert_eq!(get(&core.url("/skills")).call().unwrap().status(), 200);

    let second = core.start_sharing().unwrap();
    let new_token = second.url.split_once("?token=").unwrap().1;
    assert_ne!(old_token, new_token);
    assert_ne!(first.qr_svg, second.qr_svg);
    assert_unauthorized(get(&url(&second, &format!("/?token={old_token}"))).call());
    assert_unauthorized(
        get(&url(&second, "/skills"))
            .set("Cookie", &old_cookie)
            .call(),
    );
    pairing_cookie(&second);
}

#[test]
fn starting_again_replaces_the_active_server() {
    let home = TempDir::new().unwrap();
    let core = loopback_core(&home, Arc::new(FakeClock::new()));
    let first = core.start_sharing().unwrap();
    let second = core.start_sharing().unwrap();
    assert_ne!(first.port, second.port);
    assert_offline(&first);
    assert_eq!(core.sharing_status().as_ref(), Some(&second));
    pairing_cookie(&second);
    drop(core);
    assert_offline(&second);
}

#[test]
fn an_unpaired_server_expires_fifteen_minutes_after_start() {
    let home = TempDir::new().unwrap();
    let clock = Arc::new(FakeClock::new());
    let core = loopback_core(&home, clock.clone());
    let sharing = core.start_sharing().unwrap();

    clock.advance(Duration::from_secs(15 * 60));
    wait_for_idle_stop(&core, &sharing);
}

#[test]
fn authenticated_requests_extend_sharing_and_idle_stop_closes_the_listener() {
    let home = TempDir::new().unwrap();
    let clock = Arc::new(FakeClock::new());
    let core = loopback_core(&home, clock.clone());
    let sharing = core.start_sharing().unwrap();
    let cookie = pairing_cookie(&sharing);
    let heartbeat = url(&sharing, "/heartbeat");
    let fourteen_minutes = Duration::from_secs(14 * 60);

    clock.advance(fourteen_minutes);
    assert_eq!(
        post(&heartbeat)
            .set("Cookie", &cookie)
            .call()
            .unwrap()
            .status(),
        204
    );
    clock.advance(fourteen_minutes);
    assert_eq!(
        get(&url(&sharing, "/skills"))
            .set("Cookie", &cookie)
            .call()
            .unwrap()
            .status(),
        200
    );
    clock.advance(fourteen_minutes);
    assert_unauthorized(post(&heartbeat).call());
    clock.advance(Duration::from_secs(60));

    wait_for_idle_stop(&core, &sharing);
    assert_eq!(get(&core.url("/skills")).call().unwrap().status(), 200);
}
