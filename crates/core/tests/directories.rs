//! Ticket 09: added Skill Directories, custom harnesses, editable templates, saved settings.
#![allow(clippy::result_large_err)] // ureq::Error is large; fine in tests

use std::fs;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::path::Path;
use std::sync::Arc;

use serde_json::{json, Value};
use skills_pad_core::testing::{FakeClock, RecordingPaster};
use skills_pad_core::{Core, CoreConfig, SettingsError, SharingInfo};
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

struct Setup {
    home: TempDir,
    team: TempDir,
}

/// A home with one Claude skill, and a separate team folder with two skills.
fn setup() -> Setup {
    let home = TempDir::new().unwrap();
    write_skill(&home.path().join(".claude/skills"), "tdd");
    let team = TempDir::new().unwrap();
    write_skill(team.path(), "deploy");
    write_skill(team.path(), "release-notes");
    Setup { home, team }
}

fn start(home: &TempDir, paster: Arc<RecordingPaster>) -> Core {
    let core = Core::start(CoreConfig {
        home: home.path().to_path_buf(),
        paster,
        clock: Arc::new(FakeClock::new()),
    })
    .unwrap();
    core.set_sharing_address_override(Some(Ipv4Addr::LOCALHOST.into()));
    core
}

struct Phone {
    base: String,
    cookie: String,
}

impl Phone {
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

    fn get(&self, path: &str) -> Vec<Value> {
        ureq::get(&format!("{}{path}", self.base))
            .set("Cookie", &self.cookie)
            .call()
            .unwrap()
            .into_json()
            .unwrap()
    }

    fn tap(&self, id: &str) {
        ureq::post(&format!("{}/paste", self.base))
            .set("Cookie", &self.cookie)
            .send_json(json!({ "id": id }))
            .unwrap();
    }
}

fn ids(values: &[Value], key: &str) -> Vec<String> {
    values
        .iter()
        .map(|v| v[key].as_str().unwrap().to_string())
        .collect()
}

#[test]
fn skills_from_an_added_directory_appear_under_the_chosen_harness_on_the_phone() {
    let Setup { home, team } = setup();
    let core = start(&home, Arc::new(RecordingPaster::new()));
    let dir_id = core.add_directory(team.path(), "claude").unwrap();
    let phone = Phone::pair(&core.start_sharing().unwrap());

    let skills = phone.get("/skills");

    assert_eq!(
        ids(&skills, "id"),
        ["claude:deploy", "claude:release-notes", "claude:tdd"]
    );
    let deploy = &skills[0];
    assert_eq!(deploy["invocation"], "/deploy ");
    assert_eq!(deploy["directory"], dir_id.as_str());
    assert_eq!(skills[2]["directory"], Value::Null);

    let dirs = core.list_directories();
    assert_eq!(dirs.len(), 1);
    assert_eq!(dirs[0].skill_count, 2);
    assert_eq!(dirs[0].harness, "claude");
}

#[test]
fn a_custom_harness_gets_its_own_tab_with_its_template() {
    let Setup { home, team } = setup();
    let paster = Arc::new(RecordingPaster::new());
    let core = start(&home, paster.clone());
    let aider = core.add_custom_harness("Aider", "/run {name}").unwrap();
    core.add_directory(team.path(), &aider).unwrap();
    let phone = Phone::pair(&core.start_sharing().unwrap());

    let tabs = phone.get("/harnesses");
    assert_eq!(ids(&tabs, "id"), ["claude", aider.as_str()]);
    assert_eq!(tabs[1]["name"], "Aider");

    phone.tap(&format!("{aider}:deploy"));
    assert_eq!(paster.pasted(), ["/run deploy "]);
}

#[test]
fn editing_a_template_changes_the_text_on_the_next_tap() {
    let Setup { home, .. } = setup();
    let paster = Arc::new(RecordingPaster::new());
    let core = start(&home, paster.clone());
    let phone = Phone::pair(&core.start_sharing().unwrap());

    phone.tap("claude:tdd");
    core.set_template("claude", "Please run /{name}").unwrap();
    phone.tap("claude:tdd");
    core.reset_template("claude").unwrap();
    phone.tap("claude:tdd");

    assert_eq!(paster.pasted(), ["/tdd ", "Please run /tdd ", "/tdd "]);
}

#[test]
fn settings_survive_a_restart() {
    let Setup { home, team } = setup();
    let address: IpAddr = Ipv4Addr::LOCALHOST.into();
    let aider = {
        let core = start(&home, Arc::new(RecordingPaster::new()));
        let aider = core.add_custom_harness("Aider", "/run {name}").unwrap();
        core.add_directory(team.path(), &aider).unwrap();
        core.set_template("codex", "use ${name}").unwrap();
        core.set_sharing_address_override(Some(address));
        aider
    };

    let core = Core::start(CoreConfig {
        home: home.path().to_path_buf(),
        paster: Arc::new(RecordingPaster::new()),
        clock: Arc::new(FakeClock::new()),
    })
    .unwrap();

    assert_eq!(core.sharing_address_override(), Some(address));
    assert_eq!(core.list_directories()[0].path, team.path());
    let templates = core.list_templates();
    assert!(templates.contains(&("codex".into(), "Codex".into(), "use ${name}".into())));
    assert!(templates.contains(&(aider.clone(), "Aider".into(), "/run {name}".into())));
    assert!(core
        .list_skills()
        .iter()
        .any(|s| s.id == format!("{aider}:deploy")));
}

#[test]
fn skills_in_added_directories_are_never_cached() {
    let Setup { home, team } = setup();
    let core = start(&home, Arc::new(RecordingPaster::new()));
    core.add_directory(team.path(), "codex").unwrap();
    assert_eq!(core.list_directories()[0].skill_count, 2);

    write_skill(team.path(), "rollback");
    fs::remove_dir_all(team.path().join("deploy")).unwrap();

    let names: Vec<String> = core
        .list_skills()
        .into_iter()
        .filter(|s| s.harness == "codex")
        .map(|s| s.name)
        .collect();
    assert_eq!(names, ["release-notes", "rollback"]);
}

#[test]
fn bad_changes_are_refused_and_nothing_is_saved() {
    let Setup { home, team } = setup();
    let core = start(&home, Arc::new(RecordingPaster::new()));

    assert_eq!(
        core.add_directory(Path::new("relative/path"), "claude"),
        Err(SettingsError::NotAFolder)
    );
    assert_eq!(
        core.add_directory(&team.path().join("missing"), "claude"),
        Err(SettingsError::NotAFolder)
    );
    assert_eq!(
        core.add_directory(team.path(), "nope"),
        Err(SettingsError::UnknownHarness)
    );
    core.add_directory(team.path(), "claude").unwrap();
    assert_eq!(
        core.add_directory(team.path(), "codex"),
        Err(SettingsError::AlreadyAdded)
    );
    assert_eq!(
        core.add_custom_harness("  ", "/{name}"),
        Err(SettingsError::EmptyName)
    );
    assert_eq!(
        core.add_custom_harness("X", "no placeholder"),
        Err(SettingsError::TemplateWithoutName)
    );
    assert_eq!(
        core.set_template("claude", "broken"),
        Err(SettingsError::TemplateWithoutName)
    );

    let x = core.add_custom_harness("X", "{name}").unwrap();
    let dir = core.list_directories()[0].id.clone();
    core.remove_directory(&dir).unwrap();
    let team2 = TempDir::new().unwrap();
    core.add_directory(team2.path(), &x).unwrap();
    assert_eq!(
        core.remove_custom_harness(&x),
        Err(SettingsError::HarnessInUse)
    );
    assert_eq!(
        core.remove_directory("nope"),
        Err(SettingsError::UnknownDirectory)
    );
}
