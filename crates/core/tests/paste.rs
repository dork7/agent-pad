//! Ticket 04: the phone page, tap-to-paste and the paste queue.
#![allow(clippy::result_large_err)] // ureq::Error is large; fine in tests

use std::fs;
use std::net::{Ipv4Addr, SocketAddr};
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use serde_json::{json, Value};
use skills_pad_core::testing::{FakeClock, RecordingPaster};
use skills_pad_core::{Core, CoreConfig, PadWindow, Paster, SharingInfo};
use tempfile::TempDir;

fn write_skill(dir: &Path, name: &str) {
    let dir = dir.join(name);
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("SKILL.md"),
        format!("---\nname: {name}\ndescription: The {name} skill.\n---\n"),
    )
    .unwrap();
}

/// A home with Claude skills `tdd` and `review` and a Codex skill `plan`.
fn home() -> TempDir {
    let home = TempDir::new().unwrap();
    write_skill(&home.path().join(".claude/skills"), "tdd");
    write_skill(&home.path().join(".claude/skills"), "review");
    write_skill(&home.path().join(".codex/skills"), "plan");
    home
}

struct Phone {
    base: String,
    cookie: String,
}

impl Phone {
    /// Scans the QR code: opens the token URL and keeps the cookies (token and device ID).
    fn pair(info: &SharingInfo) -> Phone {
        let response = ureq::get(&info.url).call().unwrap();
        let cookie = response
            .all("Set-Cookie")
            .iter()
            .map(|c| c.split(';').next().unwrap())
            .collect::<Vec<_>>()
            .join("; ");
        Phone {
            base: format!("http://{}", SocketAddr::new(info.address, info.port)),
            cookie,
        }
    }

    fn get(&self, path: &str) -> Result<ureq::Response, ureq::Error> {
        ureq::get(&format!("{}{path}", self.base))
            .set("Cookie", &self.cookie)
            .call()
    }

    fn tap(&self, body: Value) -> Result<ureq::Response, ureq::Error> {
        ureq::post(&format!("{}/paste", self.base))
            .set("Cookie", &self.cookie)
            .timeout(Duration::from_secs(5))
            .send_json(body)
    }
}

fn share(home: &TempDir, paster: Arc<dyn Paster>) -> (Core, SharingInfo) {
    let core = Core::start(CoreConfig {
        home: home.path().to_path_buf(),
        paster,
        clock: Arc::new(FakeClock::new()),
    })
    .unwrap();
    core.set_sharing_address_override(Some(Ipv4Addr::LOCALHOST.into()));
    let info = core.start_sharing().unwrap();
    (core, info)
}

fn status(result: Result<ureq::Response, ureq::Error>) -> u16 {
    match result {
        Ok(r) => r.status(),
        Err(ureq::Error::Status(code, _)) => code,
        Err(e) => panic!("transport error: {e}"),
    }
}

#[test]
fn the_phone_page_and_its_data_load_from_the_scanned_url() {
    let home = home();
    let (_core, info) = share(&home, Arc::new(RecordingPaster::new()));
    let phone = Phone::pair(&info);

    let page = phone.get("/").unwrap().into_string().unwrap();
    assert!(page.contains("<title>Skills Pad</title>"));
    assert!(page.contains("fetch('/harnesses')") && page.contains("'/paste'"));

    let harnesses: Vec<Value> = phone.get("/harnesses").unwrap().into_json().unwrap();
    let ids: Vec<&str> = harnesses
        .iter()
        .map(|h| h["id"].as_str().unwrap())
        .collect();
    assert_eq!(ids, ["claude", "codex"]);

    let skills: Vec<Value> = phone.get("/skills").unwrap().into_json().unwrap();
    assert_eq!(skills.len(), 3);
    assert!(skills.iter().all(|s| s["invocation"].is_string()));
}

#[test]
fn tapping_a_skill_pastes_its_invocation_built_by_the_core() {
    let home = home();
    let paster = Arc::new(RecordingPaster::new());
    let (_core, info) = share(&home, paster.clone());
    let phone = Phone::pair(&info);

    let response: Value = phone
        .tap(json!({ "id": "claude:tdd" }))
        .unwrap()
        .into_json()
        .unwrap();
    phone.tap(json!({ "id": "codex:plan" })).unwrap();

    assert_eq!(response["pasted"], "claude:tdd");
    assert_eq!(paster.pasted(), ["/tdd ", "$plan "]);
}

#[test]
fn unknown_skills_and_arbitrary_text_are_rejected() {
    let home = home();
    let paster = Arc::new(RecordingPaster::new());
    let (_core, info) = share(&home, paster.clone());
    let phone = Phone::pair(&info);

    assert_eq!(status(phone.tap(json!({ "id": "claude:nope" }))), 404);
    assert_eq!(status(phone.tap(json!({ "text": "rm -rf ~" }))), 400);
    assert_eq!(
        status(
            ureq::post(&format!("{}/paste", phone.base))
                .set("Cookie", &phone.cookie)
                .send_string("not json")
        ),
        400
    );
    assert!(paster.pasted().is_empty());
}

#[test]
fn an_unpaired_device_cannot_paste() {
    let home = home();
    let paster = Arc::new(RecordingPaster::new());
    let (_core, info) = share(&home, paster.clone());
    let stranger = Phone {
        base: format!("http://{}", SocketAddr::new(info.address, info.port)),
        cookie: "skills_pad_token=guess".into(),
    };

    assert_eq!(status(stranger.tap(json!({ "id": "claude:tdd" }))), 401);
    assert!(paster.pasted().is_empty());
}

/// Records Pad and paste events into one shared log, so their order can be checked.
#[derive(Default)]
struct Log(Mutex<Vec<String>>);

impl Log {
    fn push(&self, event: String) {
        self.0.lock().unwrap().push(event);
    }
    fn events(&self) -> Vec<String> {
        self.0.lock().unwrap().clone()
    }
}

struct LoggingPad {
    log: Arc<Log>,
    visible: Mutex<bool>,
}

impl PadWindow for LoggingPad {
    fn hide_if_visible(&self) -> bool {
        let was_visible = std::mem::replace(&mut *self.visible.lock().unwrap(), false);
        if was_visible {
            self.log.push("hide pad".into());
        }
        was_visible
    }
}

/// A paster that takes time, logging when each paste starts and ends.
struct SlowPaster(Arc<Log>);

impl Paster for SlowPaster {
    fn paste(&self, text: &str) {
        self.0.push(format!("start {text}"));
        thread::sleep(Duration::from_millis(40));
        self.0.push(format!("end {text}"));
    }
}

#[test]
fn a_visible_pad_is_hidden_before_the_paste() {
    let home = home();
    let log = Arc::new(Log::default());
    let (core, info) = share(&home, Arc::new(SlowPaster(log.clone())));
    core.set_pad_window(Arc::new(LoggingPad {
        log: log.clone(),
        visible: Mutex::new(true),
    }));
    let phone = Phone::pair(&info);

    phone.tap(json!({ "id": "claude:tdd" })).unwrap();
    phone.tap(json!({ "id": "claude:review" })).unwrap();

    assert_eq!(
        log.events(),
        [
            "hide pad",
            "start /tdd ",
            "end /tdd ",
            "start /review ",
            "end /review "
        ]
    );
}

#[test]
fn concurrent_taps_from_two_phones_are_pasted_one_at_a_time() {
    let home = home();
    let log = Arc::new(Log::default());
    let (core, info) = share(&home, Arc::new(SlowPaster(log.clone())));
    let (a, b) = (Phone::pair(&info), Phone::pair(&info));

    let taps: Vec<_> = [(a, "claude:tdd"), (b, "codex:plan")]
        .into_iter()
        .map(|(phone, id)| {
            thread::spawn(move || {
                for _ in 0..3 {
                    phone.tap(json!({ "id": id })).unwrap();
                }
            })
        })
        .collect();
    for t in taps {
        t.join().unwrap();
    }
    // Pastes from the Pad side go through the same queue.
    core.paste_skill("claude:review").unwrap();

    let events = log.events();
    assert_eq!(events.len(), 14);
    for pair in events.chunks(2) {
        let text = pair[0].strip_prefix("start ").expect("a paste starts");
        assert_eq!(
            pair[1],
            format!("end {text}"),
            "pastes interleaved: {events:?}"
        );
    }
    assert_eq!(events[12], "start /review ");
}

#[test]
fn pastes_keep_arrival_order() {
    let home = home();
    let paster = Arc::new(RecordingPaster::new());
    let (core, info) = share(&home, paster.clone());
    let phone = Phone::pair(&info);

    for id in ["codex:plan", "claude:tdd", "claude:review", "claude:tdd"] {
        phone.tap(json!({ "id": id })).unwrap();
    }

    assert_eq!(paster.pasted(), ["$plan ", "/tdd ", "/review ", "/tdd "]);
    assert_eq!(
        core.paste_skill("claude:missing"),
        Err(skills_pad_core::PasteError::UnknownSkill)
    );
}
