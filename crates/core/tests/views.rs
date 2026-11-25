//! Ticket 11: each Phone's View of harness and directory chips.
#![allow(clippy::result_large_err)] // ureq::Error is large; fine in tests

use std::fs;
use std::net::{Ipv4Addr, SocketAddr};
use std::path::Path;
use std::sync::Arc;

use serde_json::{json, Value};
use skills_pad_core::testing::{FakeClock, RecordingPaster};
use skills_pad_core::{Core, CoreConfig, PhoneError, SharingInfo};
use tempfile::TempDir;

fn write_skill(dir: &Path, name: &str) {
    let dir = dir.join(name);
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("SKILL.md"), format!("---\nname: {name}\n---\n")).unwrap();
}

struct World {
    home: TempDir,
    _team: TempDir,
    core: Core,
    paster: Arc<RecordingPaster>,
    team_dir: String,
    info: SharingInfo,
}

/// Claude `tdd`, Codex `plan`, and an added `team` directory (assigned to Claude) with `deploy`.
fn world() -> World {
    let home = TempDir::new().unwrap();
    write_skill(&home.path().join(".claude/skills"), "tdd");
    write_skill(&home.path().join(".codex/skills"), "plan");
    let team = TempDir::new().unwrap();
    write_skill(team.path(), "deploy");
    let paster = Arc::new(RecordingPaster::new());
    let core = Core::start(CoreConfig {
        home: home.path().to_path_buf(),
        paster: paster.clone(),
        clock: Arc::new(FakeClock::new()),
    })
    .unwrap();
    core.set_sharing_address_override(Some(Ipv4Addr::LOCALHOST.into()));
    let team_dir = core.add_directory(team.path(), "claude").unwrap();
    let info = core.start_sharing().unwrap();
    World {
        home,
        _team: team,
        core,
        paster,
        team_dir,
        info,
    }
}

struct Phone {
    base: String,
    cookie: String,
    id: String,
}

impl Phone {
    fn pair(core: &Core, info: &SharingInfo) -> Phone {
        let response = ureq::get(&info.url).call().unwrap();
        let cookies: Vec<String> = response
            .all("Set-Cookie")
            .iter()
            .map(|c| c.split(';').next().unwrap().to_string())
            .collect();
        let id = cookies
            .iter()
            .find_map(|c| c.strip_prefix("skills_pad_device="))
            .unwrap()
            .to_string();
        assert!(core.list_phones().iter().any(|p| p.id == id));
        Phone {
            base: format!("http://{}", SocketAddr::new(info.address, info.port)),
            cookie: cookies.join("; "),
            id,
        }
    }

    fn get(&self, path: &str) -> Value {
        ureq::get(&format!("{}{path}", self.base))
            .set("Cookie", &self.cookie)
            .call()
            .unwrap()
            .into_json()
            .unwrap()
    }

    fn skill_ids(&self) -> Vec<String> {
        self.get("/skills")
            .as_array()
            .unwrap()
            .iter()
            .map(|s| s["id"].as_str().unwrap().to_string())
            .collect()
    }

    fn tabs(&self) -> Vec<String> {
        self.get("/harnesses")
            .as_array()
            .unwrap()
            .iter()
            .map(|h| h["id"].as_str().unwrap().to_string())
            .collect()
    }

    fn tap(&self, id: &str) -> u16 {
        match ureq::post(&format!("{}/paste", self.base))
            .set("Cookie", &self.cookie)
            .send_json(json!({ "id": id }))
        {
            Ok(r) => r.status(),
            Err(ureq::Error::Status(code, _)) => code,
            Err(e) => panic!("{e}"),
        }
    }
}

#[test]
fn new_phones_see_everything() {
    let w = world();
    let phone = Phone::pair(&w.core, &w.info);

    assert_eq!(
        phone.skill_ids(),
        ["claude:deploy", "claude:tdd", "codex:plan"]
    );
    assert_eq!(phone.get("/view"), Value::Null);
    assert_eq!(
        w.core.list_chips(),
        ["h:claude", "h:codex", &format!("d:{}", w.team_dir)]
    );
}

#[test]
fn a_harness_chip_gives_all_of_that_harness_and_a_directory_chip_only_its_folder() {
    let w = world();
    let phone = Phone::pair(&w.core, &w.info);

    w.core
        .set_phone_view(&phone.id, Some(vec!["h:codex".into()]))
        .unwrap();
    assert_eq!(phone.skill_ids(), ["codex:plan"]);
    assert_eq!(phone.tabs(), ["codex"]);

    w.core
        .set_phone_view(&phone.id, Some(vec![format!("d:{}", w.team_dir)]))
        .unwrap();
    assert_eq!(phone.skill_ids(), ["claude:deploy"]);

    w.core.add_chip_to_phone(&phone.id, "h:claude").unwrap();
    assert_eq!(phone.skill_ids(), ["claude:deploy", "claude:tdd"]);
}

#[test]
fn removing_a_chip_updates_the_phone_live() {
    let w = world();
    let phone = Phone::pair(&w.core, &w.info);

    // From "everything", × on one chip keeps all the others.
    w.core
        .remove_chip_from_phone(&phone.id, "h:claude")
        .unwrap();
    assert_eq!(phone.skill_ids(), ["claude:deploy", "codex:plan"]);
    assert_eq!(
        phone.get("/view"),
        json!(["h:codex", format!("d:{}", w.team_dir)])
    );

    w.core
        .remove_chip_from_phone(&phone.id, &format!("d:{}", w.team_dir))
        .unwrap();
    assert_eq!(phone.skill_ids(), ["codex:plan"]);
}

#[test]
fn a_phone_with_nothing_assigned_gets_an_empty_list_and_an_empty_view() {
    let w = world();
    let phone = Phone::pair(&w.core, &w.info);

    w.core.set_phone_view(&phone.id, Some(vec![])).unwrap();

    assert!(phone.skill_ids().is_empty());
    assert!(phone.tabs().is_empty());
    assert_eq!(phone.get("/view"), json!([]));
    let page = ureq::get(&format!("{}/", phone.base))
        .set("Cookie", &phone.cookie)
        .call()
        .unwrap()
        .into_string()
        .unwrap();
    assert!(page.contains("Nothing is assigned to this phone yet."));
}

#[test]
fn a_phone_cannot_paste_outside_its_view() {
    let w = world();
    let (a, b) = (Phone::pair(&w.core, &w.info), Phone::pair(&w.core, &w.info));
    w.core
        .set_phone_view(&a.id, Some(vec!["h:codex".into()]))
        .unwrap();

    assert_eq!(a.tap("claude:tdd"), 403);
    assert_eq!(a.tap("codex:plan"), 200);
    assert_eq!(b.tap("claude:tdd"), 200, "other phones are unaffected");
    assert_eq!(w.paster.pasted(), ["$plan ", "/tdd "]);
}

#[test]
fn views_survive_restarts() {
    let w = world();
    let phone = Phone::pair(&w.core, &w.info);
    w.core
        .set_phone_view(&phone.id, Some(vec!["h:codex".into(), "h:codex".into()]))
        .unwrap();
    let home = w.home;
    drop(w.core);

    let core = Core::start(CoreConfig {
        home: home.path().to_path_buf(),
        paster: Arc::new(RecordingPaster::new()),
        clock: Arc::new(FakeClock::new()),
    })
    .unwrap();
    let saved = core.list_phones();
    assert_eq!(saved[0].view, Some(vec!["h:codex".to_string()]));
}

#[test]
fn unknown_chips_and_removed_directories() {
    let w = world();
    let phone = Phone::pair(&w.core, &w.info);

    assert_eq!(
        w.core.add_chip_to_phone(&phone.id, "h:nope"),
        Err(PhoneError::UnknownChip)
    );
    assert_eq!(
        w.core.set_phone_view(&phone.id, Some(vec!["bogus".into()])),
        Err(PhoneError::UnknownChip)
    );
    w.core
        .set_phone_view(&phone.id, Some(vec![format!("d:{}", w.team_dir)]))
        .unwrap();
    w.core.remove_directory(&w.team_dir).unwrap();
    assert_eq!(w.core.list_phones()[0].view, Some(vec![]));
    assert!(phone.skill_ids().is_empty());
}
