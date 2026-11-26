//! Ticket 12: Profiles of every Phone's View.

use std::fs;
use std::net::Ipv4Addr;
use std::path::Path;
use std::sync::Arc;

use agent_pad_core::testing::{FakeClock, RecordingPaster};
use agent_pad_core::{Core, CoreConfig, ProfileError};
use tempfile::TempDir;

fn write_skill(dir: &Path, name: &str) {
    let dir = dir.join(name);
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("SKILL.md"), format!("---\nname: {name}\n---\n")).unwrap();
}

fn start(home: &TempDir) -> Core {
    let core = Core::start(CoreConfig {
        home: home.path().to_path_buf(),
        paster: Arc::new(RecordingPaster::new()),
        clock: Arc::new(FakeClock::new()),
    })
    .unwrap();
    core.set_sharing_address_override(Some(Ipv4Addr::LOCALHOST.into()));
    core
}

/// Pairs a phone and returns its device ID.
fn pair(core: &Core) -> String {
    let info = core
        .sharing_status()
        .unwrap_or_else(|| core.start_sharing().unwrap());
    let response = ureq::get(&info.url).call().unwrap();
    response
        .all("Set-Cookie")
        .iter()
        .find_map(|c| {
            c.split(';')
                .next()
                .unwrap()
                .strip_prefix("skills_pad_device=")
        })
        .unwrap()
        .to_string()
}

fn home() -> TempDir {
    let home = TempDir::new().unwrap();
    write_skill(&home.path().join(".claude/skills"), "tdd");
    write_skill(&home.path().join(".codex/skills"), "plan");
    home
}

fn view(core: &Core, id: &str) -> Option<Vec<String>> {
    core.list_phones()
        .into_iter()
        .find(|p| p.id == id)
        .unwrap()
        .view
}

fn chips(list: &[&str]) -> Option<Vec<String>> {
    Some(list.iter().map(|c| c.to_string()).collect())
}

#[test]
fn saving_captures_every_phone_and_switching_applies_all_at_once() {
    let home = home();
    let core = start(&home);
    let (iphone, galaxy) = (pair(&core), pair(&core));

    core.save_profile("Everything").unwrap();
    core.set_phone_view(&iphone, chips(&["h:claude"])).unwrap();
    core.set_phone_view(&galaxy, chips(&["h:codex"])).unwrap();
    core.save_profile("Split").unwrap();

    core.switch_profile("Everything").unwrap();
    assert_eq!(view(&core, &iphone), None);
    assert_eq!(view(&core, &galaxy), None);

    core.switch_profile("Split").unwrap();
    assert_eq!(view(&core, &iphone), chips(&["h:claude"]));
    assert_eq!(view(&core, &galaxy), chips(&["h:codex"]));

    let (profiles, active) = core.list_profiles();
    let names: Vec<&str> = profiles.iter().map(|p| p.name.as_str()).collect();
    assert_eq!(names, ["Everything", "Split"]);
    assert_eq!(active.as_deref(), Some("Split"));
}

#[test]
fn entries_for_unknown_phones_are_skipped_and_unmentioned_phones_are_unchanged() {
    let home = home();
    let core = start(&home);
    let gone = pair(&core);
    core.set_phone_view(&gone, chips(&["h:codex"])).unwrap();
    core.save_profile("Old").unwrap();
    core.forget_phone(&gone).unwrap();

    let fresh = pair(&core);
    core.set_phone_view(&fresh, chips(&["h:claude"])).unwrap();

    core.switch_profile("Old").unwrap();

    assert_eq!(view(&core, &fresh), chips(&["h:claude"]));
    assert_eq!(core.list_phones().len(), 1);
}

#[test]
fn chips_that_no_longer_exist_are_dropped_on_switch() {
    let home = home();
    let team = TempDir::new().unwrap();
    write_skill(team.path(), "deploy");
    let core = start(&home);
    let dir = core.add_directory(team.path(), "claude").unwrap();
    let phone = pair(&core);
    core.set_phone_view(&phone, chips(&["h:codex", &format!("d:{dir}")]))
        .unwrap();
    core.save_profile("With team").unwrap();

    core.remove_directory(&dir).unwrap();
    core.set_phone_view(&phone, None).unwrap();
    core.switch_profile("With team").unwrap();

    assert_eq!(view(&core, &phone), chips(&["h:codex"]));
}

#[test]
fn profiles_can_be_renamed_and_deleted_and_persist() {
    let home = home();
    {
        let core = start(&home);
        pair(&core);
        core.save_profile("A").unwrap();
        core.save_profile("B").unwrap();
        core.rename_profile("B", "Work").unwrap();
        assert_eq!(
            core.rename_profile("A", "Work"),
            Err(ProfileError::NameTaken)
        );
        assert_eq!(core.rename_profile("A", " "), Err(ProfileError::EmptyName));
        core.delete_profile("A").unwrap();
        assert_eq!(core.delete_profile("A"), Err(ProfileError::UnknownProfile));
        assert_eq!(core.switch_profile("A"), Err(ProfileError::UnknownProfile));
        assert_eq!(core.save_profile(""), Err(ProfileError::EmptyName));
    }

    let core = start(&home);
    let (profiles, active) = core.list_profiles();
    assert_eq!(profiles.len(), 1);
    assert_eq!(profiles[0].name, "Work");
    assert_eq!(active.as_deref(), Some("Work"));
}
