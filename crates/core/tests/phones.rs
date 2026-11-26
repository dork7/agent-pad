//! Ticket 10: several Phones, recognized by device ID, renamed, disconnected and forgotten.
#![allow(clippy::result_large_err)] // ureq::Error is large; fine in tests

use std::collections::BTreeMap;
use std::fs;
use std::net::{Ipv4Addr, SocketAddr};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde_json::json;
use agent_pad_core::testing::{FakeClock, RecordingPaster};
use agent_pad_core::{Core, CoreConfig, PhoneError, SharingInfo};
use tempfile::TempDir;

const IPHONE: &str = "Mozilla/5.0 (iPhone; CPU iPhone OS 18_0 like Mac OS X) AppleWebKit/605.1.15";
const SAMSUNG: &str = "Mozilla/5.0 (Linux; Android 14; SM-S918B Build/UP1A) AppleWebKit/537.36";

/// A phone browser with a cookie jar.
struct Browser {
    user_agent: &'static str,
    jar: Mutex<BTreeMap<String, String>>,
    base: Mutex<String>,
}

impl Browser {
    fn new(user_agent: &'static str) -> Self {
        Browser {
            user_agent,
            jar: Mutex::default(),
            base: Mutex::default(),
        }
    }

    fn cookies(&self) -> String {
        let jar = self.jar.lock().unwrap();
        jar.iter()
            .map(|(k, v)| format!("{k}={v}"))
            .collect::<Vec<_>>()
            .join("; ")
    }

    fn device_id(&self) -> Option<String> {
        self.jar.lock().unwrap().get("skills_pad_device").cloned()
    }

    fn keep_cookies(&self, response: &ureq::Response) {
        let mut jar = self.jar.lock().unwrap();
        for c in response.all("Set-Cookie") {
            let (name, value) = c.split(';').next().unwrap().split_once('=').unwrap();
            jar.insert(name.into(), value.into());
        }
    }

    fn send(&self, request: ureq::Request) -> Result<ureq::Response, ureq::Error> {
        let request = request
            .set("User-Agent", self.user_agent)
            .set("Cookie", &self.cookies())
            .timeout(Duration::from_secs(5));
        let response = request.call()?;
        self.keep_cookies(&response);
        Ok(response)
    }

    /// Scans the QR code shown by the Pad.
    fn scan(&self, info: &SharingInfo) -> u16 {
        *self.base.lock().unwrap() = format!("http://{}", SocketAddr::new(info.address, info.port));
        status(self.send(ureq::get(&info.url)))
    }

    fn get(&self, path: &str) -> u16 {
        let url = format!("{}{path}", self.base.lock().unwrap());
        status(self.send(ureq::get(&url)))
    }

    fn tap(&self, id: &str) -> u16 {
        let url = format!("{}/paste", self.base.lock().unwrap());
        let request = ureq::post(&url)
            .set("User-Agent", self.user_agent)
            .set("Cookie", &self.cookies())
            .timeout(Duration::from_secs(5));
        status(request.send_json(json!({ "id": id })))
    }
}

fn status(result: Result<ureq::Response, ureq::Error>) -> u16 {
    match result {
        Ok(r) => r.status(),
        Err(ureq::Error::Status(code, _)) => code,
        Err(e) => panic!("transport error: {e}"),
    }
}

fn home() -> TempDir {
    let home = TempDir::new().unwrap();
    let dir = home.path().join(".claude/skills/tdd");
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("SKILL.md"), "---\nname: tdd\n---\n").unwrap();
    home
}

fn start(home: &TempDir, clock: Arc<FakeClock>) -> Core {
    let core = Core::start(CoreConfig {
        home: home.path().to_path_buf(),
        paster: Arc::new(RecordingPaster::new()),
        clock,
    })
    .unwrap();
    core.set_sharing_address_override(Some(Ipv4Addr::LOCALHOST.into()));
    core
}

#[test]
fn two_phones_share_one_session_and_are_named_from_their_browser() {
    let home = home();
    let core = start(&home, Arc::new(FakeClock::new()));
    let info = core.start_sharing().unwrap();
    let (iphone, samsung) = (Browser::new(IPHONE), Browser::new(SAMSUNG));

    assert_eq!(iphone.scan(&info), 200);
    assert_eq!(samsung.scan(&info), 200);
    assert_eq!(iphone.tap("claude:tdd"), 200);
    assert_eq!(samsung.tap("claude:tdd"), 200);

    let phones = core.list_phones();
    let names: Vec<&str> = phones.iter().map(|p| p.name.as_str()).collect();
    assert_eq!(names, ["iPhone", "SM-S918B"]);
    assert!(phones.iter().all(|p| p.online && p.last_seen > 0));
    assert_ne!(iphone.device_id(), samsung.device_id());
    assert_eq!(phones[0].id, iphone.device_id().unwrap());
}

#[test]
fn a_known_phone_is_recognized_in_a_new_sharing_session() {
    let home = home();
    let core = start(&home, Arc::new(FakeClock::new()));
    let iphone = Browser::new(IPHONE);
    iphone.scan(&core.start_sharing().unwrap());
    let id = iphone.device_id().unwrap();

    core.stop_sharing();
    let info = core.start_sharing().unwrap();
    // The old token cookie no longer works on the new server; scanning the new QR does.
    *iphone.base.lock().unwrap() = format!("http://{}", SocketAddr::new(info.address, info.port));
    assert_eq!(iphone.get("/skills"), 401);
    assert_eq!(iphone.scan(&info), 200);

    assert_eq!(iphone.device_id().unwrap(), id);
    let phones = core.list_phones();
    assert_eq!(phones.len(), 1);
    assert_eq!(phones[0].id, id);
}

#[test]
fn a_disconnected_phone_is_refused_until_it_scans_again() {
    let home = home();
    let core = start(&home, Arc::new(FakeClock::new()));
    let info = core.start_sharing().unwrap();
    let (iphone, samsung) = (Browser::new(IPHONE), Browser::new(SAMSUNG));
    iphone.scan(&info);
    samsung.scan(&info);

    core.disconnect_phone(&iphone.device_id().unwrap()).unwrap();

    assert_eq!(iphone.get("/skills"), 401);
    assert_eq!(iphone.tap("claude:tdd"), 401);
    assert_eq!(samsung.tap("claude:tdd"), 200);
    let phones = core.list_phones();
    assert_eq!(phones.len(), 2, "disconnect keeps the phone");
    assert!(!phones[0].online);

    assert_eq!(iphone.scan(&info), 200);
    assert_eq!(iphone.tap("claude:tdd"), 200);
    assert!(core.list_phones()[0].online);
}

#[test]
fn forget_removes_the_phone_and_it_comes_back_as_new_only_by_scanning() {
    let home = home();
    let core = start(&home, Arc::new(FakeClock::new()));
    let info = core.start_sharing().unwrap();
    let iphone = Browser::new(IPHONE);
    iphone.scan(&info);
    let old_id = iphone.device_id().unwrap();
    core.rename_phone(&old_id, "Work phone").unwrap();

    core.forget_phone(&old_id).unwrap();

    assert!(core.list_phones().is_empty());
    assert_eq!(iphone.get("/skills"), 401);
    assert_eq!(iphone.scan(&info), 200);
    let phones = core.list_phones();
    assert_eq!(phones.len(), 1);
    assert_ne!(phones[0].id, old_id);
    assert_eq!(phones[0].name, "iPhone", "the old name is gone");
    assert_eq!(phones[0].view, None, "a new phone sees everything");
}

#[test]
fn online_state_follows_recent_activity() {
    let home = home();
    let clock = Arc::new(FakeClock::new());
    let core = start(&home, clock.clone());
    let info = core.start_sharing().unwrap();
    let iphone = Browser::new(IPHONE);
    iphone.scan(&info);
    assert!(core.list_phones()[0].online);

    clock.advance(Duration::from_secs(80));
    assert!(!core.list_phones()[0].online);

    assert_eq!(iphone.get("/skills"), 200);
    assert!(core.list_phones()[0].online);
}

#[test]
fn names_and_phones_persist_across_restarts() {
    let home = home();
    let id = {
        let core = start(&home, Arc::new(FakeClock::new()));
        let iphone = Browser::new(IPHONE);
        iphone.scan(&core.start_sharing().unwrap());
        let id = iphone.device_id().unwrap();
        core.rename_phone(&id, "  Desk phone  ").unwrap();
        core.rename_phone(&id, "   ").unwrap(); // ignored
        id
    };

    let core = start(&home, Arc::new(FakeClock::new()));
    let phones = core.list_phones();
    assert_eq!(phones.len(), 1);
    assert_eq!(phones[0].id, id);
    assert_eq!(phones[0].name, "Desk phone");
    assert!(!phones[0].online);
}

#[test]
fn unknown_phone_ids_are_reported() {
    let home = home();
    let core = start(&home, Arc::new(FakeClock::new()));
    assert_eq!(
        core.rename_phone("nope", "x"),
        Err(PhoneError::UnknownPhone)
    );
    assert_eq!(core.disconnect_phone("nope"), Err(PhoneError::UnknownPhone));
    assert_eq!(core.forget_phone("nope"), Err(PhoneError::UnknownPhone));
}
