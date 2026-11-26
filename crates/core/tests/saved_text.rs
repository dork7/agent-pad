//! Tickets 16 and 17: argument presets on skills, and snippets.
#![allow(clippy::result_large_err)] // ureq::Error is large; fine in tests

use std::fs;
use std::net::{Ipv4Addr, SocketAddr};
use std::path::Path;
use std::sync::Arc;

use serde_json::{json, Value};
use agent_pad_core::testing::{FakeClock, RecordingPaster};
use agent_pad_core::{Core, CoreConfig, SettingsError, SNIPPETS_CHIP};
use tempfile::TempDir;

fn write_skill(dir: &Path, name: &str) {
    let dir = dir.join(name);
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("SKILL.md"), format!("---\nname: {name}\n---\n")).unwrap();
}

struct World {
    home: TempDir,
    core: Core,
    paster: Arc<RecordingPaster>,
    phone: Phone,
}

struct Phone {
    base: String,
    cookie: String,
    id: String,
}

impl Phone {
    fn get(&self, path: &str) -> Value {
        ureq::get(&format!("{}{path}", self.base))
            .set("Cookie", &self.cookie)
            .call()
            .unwrap()
            .into_json()
            .unwrap()
    }

    fn post(&self, path: &str, body: Value) -> u16 {
        match ureq::post(&format!("{}{path}", self.base))
            .set("Cookie", &self.cookie)
            .send_json(body)
        {
            Ok(r) => r.status(),
            Err(ureq::Error::Status(code, _)) => code,
            Err(e) => panic!("{e}"),
        }
    }
}

fn start(home: &Path, paster: Arc<RecordingPaster>) -> Core {
    let core = Core::start(CoreConfig {
        home: home.to_path_buf(),
        paster,
        clock: Arc::new(FakeClock::new()),
    })
    .unwrap();
    core.set_sharing_address_override(Some(Ipv4Addr::LOCALHOST.into()));
    core
}

fn world() -> World {
    let home = TempDir::new().unwrap();
    write_skill(&home.path().join(".claude/skills"), "review");
    write_skill(&home.path().join(".codex/skills"), "plan");
    let paster = Arc::new(RecordingPaster::new());
    let core = start(home.path(), paster.clone());
    let info = core.start_sharing().unwrap();
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
    let phone = Phone {
        base: format!("http://{}", SocketAddr::new(info.address, info.port)),
        cookie: cookies.join("; "),
        id,
    };
    World {
        home,
        core,
        paster,
        phone,
    }
}

fn presets_on_phone(phone: &Phone, skill: &str) -> Vec<(String, String)> {
    phone
        .get("/skills")
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["id"] == skill)
        .unwrap()["presets"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| {
            (
                p["id"].as_str().unwrap().to_string(),
                p["text"].as_str().unwrap().to_string(),
            )
        })
        .collect()
}

#[test]
fn presets_show_beside_the_skill_and_paste_the_full_line() {
    let w = world();
    let high = w.core.add_preset("claude:review", "  high  ").unwrap();
    w.core
        .add_preset("claude:review", "focus on tests")
        .unwrap();

    let presets = presets_on_phone(&w.phone, "claude:review");
    assert_eq!(presets[0], (high.clone(), "high".to_string()));
    assert_eq!(presets.len(), 2);
    assert!(presets_on_phone(&w.phone, "codex:plan").is_empty());

    assert_eq!(w.phone.post("/paste", json!({"id": "claude:review"})), 200);
    assert_eq!(
        w.phone
            .post("/paste", json!({"id": "claude:review", "preset": high})),
        200
    );
    assert_eq!(w.paster.pasted(), ["/review ", "/review high "]);
}

#[test]
fn presets_are_edited_deleted_live_and_only_ids_are_accepted() {
    let w = world();
    let id = w.core.add_preset("claude:review", "high").unwrap();
    w.core.update_preset(&id, "low").unwrap();
    assert_eq!(presets_on_phone(&w.phone, "claude:review")[0].1, "low");

    assert_eq!(
        w.phone
            .post("/paste", json!({"id": "codex:plan", "preset": id})),
        404,
        "a preset of another skill"
    );
    assert_eq!(
        w.phone
            .post("/paste", json!({"id": "claude:review", "text": "rm -rf"})),
        200,
        "unknown fields are ignored; only the bare invocation is pasted"
    );
    w.core.delete_preset(&id).unwrap();
    assert!(presets_on_phone(&w.phone, "claude:review").is_empty());
    assert_eq!(
        w.phone
            .post("/paste", json!({"id": "claude:review", "preset": id})),
        404
    );
    assert_eq!(w.paster.pasted(), ["/review "]);
    assert_eq!(
        w.core.add_preset("claude:review", " "),
        Err(SettingsError::EmptyText)
    );
}

#[test]
fn presets_belong_to_a_skill_by_name_and_survive_rescans_and_restarts() {
    let w = world();
    w.core.add_preset("claude:review", "high").unwrap();
    // The skill disappears and comes back.
    let skill_dir = w.home.path().join(".claude/skills/review");
    fs::remove_dir_all(&skill_dir).unwrap();
    assert!(w.core.list_skills().iter().all(|s| s.id != "claude:review"));
    write_skill(&w.home.path().join(".claude/skills"), "review");
    let home = w.home;
    drop(w.core);

    let core = start(home.path(), Arc::new(RecordingPaster::new()));
    let review = core
        .list_skills()
        .into_iter()
        .find(|s| s.id == "claude:review")
        .unwrap();
    assert_eq!(review.presets[0].text, "high");
}

#[test]
fn snippets_are_managed_in_order_and_pasted_exactly_as_saved() {
    let w = world();
    let commit = w
        .core
        .add_snippet("Commit", "feat: \n\nCo-authored-by: me")
        .unwrap();
    let path = w.core.add_snippet("Path", "~/work/app").unwrap();
    w.core.move_snippet(&path, 0).unwrap();
    w.core
        .update_snippet(&commit, "Commit msg", "fix: ")
        .unwrap();

    let titles: Vec<String> = w
        .phone
        .get("/snippets")
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["title"].as_str().unwrap().to_string())
        .collect();
    assert_eq!(titles, ["Path", "Commit msg"]);

    assert_eq!(w.phone.post("/paste-snippet", json!({"id": commit})), 200);
    assert_eq!(w.phone.post("/paste-snippet", json!({"id": "nope"})), 404);
    assert_eq!(w.phone.post("/paste-snippet", json!({"text": "x"})), 400);
    assert_eq!(w.paster.pasted(), ["fix: "]);

    w.core.delete_snippet(&path).unwrap();
    assert_eq!(w.core.list_snippets().len(), 1);
}

#[test]
fn the_snippets_chip_controls_the_tab_and_profiles_restore_it() {
    let w = world();
    let id = w.core.add_snippet("Path", "~/work/app").unwrap();
    assert!(w.core.list_chips().contains(&SNIPPETS_CHIP.to_string()));

    w.core
        .set_phone_view(&w.phone.id, Some(vec!["h:claude".into()]))
        .unwrap();
    w.core.save_profile("No snippets").unwrap();
    assert_eq!(w.phone.get("/snippets"), json!([]));
    assert_eq!(w.phone.post("/paste-snippet", json!({"id": id})), 403);

    w.core
        .add_chip_to_phone(&w.phone.id, SNIPPETS_CHIP)
        .unwrap();
    w.core.save_profile("With snippets").unwrap();
    assert_eq!(w.phone.get("/snippets").as_array().unwrap().len(), 1);

    w.core.switch_profile("No snippets").unwrap();
    assert_eq!(w.phone.get("/snippets"), json!([]));
    w.core.switch_profile("With snippets").unwrap();
    assert_eq!(w.phone.post("/paste-snippet", json!({"id": id})), 200);
    assert_eq!(w.paster.pasted(), ["~/work/app"]);
}

#[test]
fn snippets_persist_across_restarts() {
    let w = world();
    w.core.add_snippet("A", "one").unwrap();
    w.core.add_snippet("B", "two").unwrap();
    let home = w.home;
    drop(w.core);

    let core = start(home.path(), Arc::new(RecordingPaster::new()));
    let texts: Vec<String> = core.list_snippets().into_iter().map(|s| s.text).collect();
    assert_eq!(texts, ["one", "two"]);
}
