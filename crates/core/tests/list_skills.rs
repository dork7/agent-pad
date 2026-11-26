//! Ticket 01: list Claude skills through the Core's HTTP API.

use std::fs;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use serde_json::Value;
use agent_pad_core::testing::{FakeClock, RecordingPaster};
use agent_pad_core::{Clock, Core, CoreConfig, Paster};
use tempfile::TempDir;

fn write_skill(dir: &Path, name: &str, description: &str) {
    let dir = dir.join(name);
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("SKILL.md"),
        format!("---\nname: {name}\ndescription: {description}\n---\n\n# {name}\n"),
    )
    .unwrap();
}

fn start(home: &TempDir) -> Core {
    Core::start(CoreConfig {
        home: home.path().to_path_buf(),
        paster: Arc::new(RecordingPaster::new()),
        clock: Arc::new(FakeClock::new()),
    })
    .unwrap()
}

fn get_skills(core: &Core) -> Vec<Value> {
    ureq::get(&core.url("/skills"))
        .call()
        .unwrap()
        .into_json()
        .unwrap()
}

fn names(skills: &[Value]) -> Vec<&str> {
    skills.iter().map(|s| s["name"].as_str().unwrap()).collect()
}

#[test]
fn lists_global_claude_skills_with_name_and_description() {
    let home = TempDir::new().unwrap();
    let skills_dir = home.path().join(".claude/skills");
    write_skill(&skills_dir, "tdd", "Test-driven development.");
    write_skill(&skills_dir, "grilling", "Stress-test a plan.");

    let core = start(&home);
    let skills = get_skills(&core);

    assert_eq!(names(&skills), ["grilling", "tdd"]);
    let tdd = &skills[1];
    assert_eq!(tdd["id"], "claude:tdd");
    assert_eq!(tdd["harness"], "claude");
    assert_eq!(tdd["description"], "Test-driven development.");
}

#[test]
fn description_is_reduced_to_one_line() {
    let home = TempDir::new().unwrap();
    let dir = home.path().join(".claude/skills/review");
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("SKILL.md"),
        "---\nname: review\ndescription: |\n  Review a branch.\n  Runs two sub-agents.\n---\n",
    )
    .unwrap();

    let skills = get_skills(&start(&home));

    assert_eq!(skills[0]["description"], "Review a branch.");
}

#[test]
fn plugin_skills_appear_once_from_the_newest_version() {
    let home = TempDir::new().unwrap();
    let plugin = home
        .path()
        .join(".claude/plugins/cache/official/superpowers");
    write_skill(
        &plugin.join("4.9.2/skills"),
        "brainstorming",
        "Old description.",
    );
    write_skill(
        &plugin.join("4.10.0/skills"),
        "brainstorming",
        "New description.",
    );
    write_skill(
        &plugin.join("4.9.2/skills"),
        "removed-later",
        "Only in the old version.",
    );

    let skills = get_skills(&start(&home));

    assert_eq!(names(&skills), ["superpowers:brainstorming"]);
    assert_eq!(skills[0]["id"], "claude:superpowers:brainstorming");
    assert_eq!(skills[0]["description"], "New description.");
}

#[test]
fn folders_without_skill_md_are_ignored_and_missing_names_fall_back_to_the_folder() {
    let home = TempDir::new().unwrap();
    let skills_dir = home.path().join(".claude/skills");
    fs::create_dir_all(skills_dir.join("not-a-skill")).unwrap();
    fs::create_dir_all(skills_dir.join("unnamed")).unwrap();
    fs::write(skills_dir.join("unnamed/SKILL.md"), "# No frontmatter\n").unwrap();

    let skills = get_skills(&start(&home));

    assert_eq!(names(&skills), ["unnamed"]);
    assert_eq!(skills[0]["description"], "");
}

#[test]
fn skills_are_rescanned_on_every_request() {
    let home = TempDir::new().unwrap();
    let core = start(&home);
    assert!(get_skills(&core).is_empty());

    write_skill(
        &home.path().join(".claude/skills"),
        "tdd",
        "Test-driven development.",
    );

    assert_eq!(names(&get_skills(&core)), ["tdd"]);
}

#[test]
fn each_core_gets_its_own_random_port_and_unknown_paths_are_404() {
    let (a, b) = (TempDir::new().unwrap(), TempDir::new().unwrap());
    let (core_a, core_b) = (start(&a), start(&b));
    assert_ne!(core_a.addr().port(), core_b.addr().port());

    match ureq::get(&core_a.url("/nope")).call() {
        Err(ureq::Error::Status(404, _)) => {}
        other => panic!("expected 404, got {other:?}"),
    }
}

#[test]
fn list_skills_command_matches_the_http_api() {
    let home = TempDir::new().unwrap();
    write_skill(
        &home.path().join(".claude/skills"),
        "tdd",
        "Test-driven development.",
    );
    let core = start(&home);

    let from_command: Vec<String> = core.list_skills().into_iter().map(|s| s.id).collect();
    let from_http: Vec<String> = get_skills(&core)
        .iter()
        .map(|s| s["id"].as_str().unwrap().to_string())
        .collect();

    assert_eq!(from_command, from_http);
}

#[test]
fn test_fakes_record_pastes_and_move_time_on_demand() {
    let paster = RecordingPaster::new();
    paster.paste("/tdd ");
    paster.paste("$review ");
    assert_eq!(paster.pasted(), ["/tdd ", "$review "]);

    let clock = FakeClock::new();
    let t0 = clock.now();
    assert_eq!(clock.now(), t0);
    clock.advance(Duration::from_secs(15 * 60));
    assert_eq!(clock.now() - t0, Duration::from_secs(900));
}
