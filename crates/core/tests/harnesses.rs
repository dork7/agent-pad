//! Ticket 02: harness discovery, the Shared chip and Invocation templates.

use std::fs;
use std::path::Path;
use std::sync::Arc;

use serde_json::Value;
use skills_pad_core::harness::REGISTRY;
use skills_pad_core::testing::{FakeClock, RecordingPaster};
use skills_pad_core::{Core, CoreConfig};
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

fn start(home: &TempDir) -> Core {
    Core::start(CoreConfig {
        home: home.path().to_path_buf(),
        paster: Arc::new(RecordingPaster::new()),
        clock: Arc::new(FakeClock::new()),
    })
    .unwrap()
}

fn get(core: &Core, path: &str) -> Vec<Value> {
    ureq::get(&core.url(path))
        .call()
        .unwrap()
        .into_json()
        .unwrap()
}

fn skill<'a>(skills: &'a [Value], id: &str) -> &'a Value {
    skills
        .iter()
        .find(|s| s["id"] == id)
        .unwrap_or_else(|| panic!("no skill {id} in {skills:?}"))
}

fn dir_of(harness: &str) -> &'static str {
    REGISTRY
        .iter()
        .find(|h| h.id == harness)
        .unwrap()
        .skill_dirs[0]
}

#[test]
fn harnesses_with_no_skills_are_not_listed() {
    let home = TempDir::new().unwrap();
    write_skill(&home.path().join(dir_of("claude")), "tdd");
    write_skill(&home.path().join(dir_of("codex")), "review");
    write_skill(&home.path().join(dir_of("codex")), "plan");
    // An empty folder for another harness does not make it appear.
    fs::create_dir_all(home.path().join(dir_of("cursor"))).unwrap();

    let core = start(&home);
    let chips = get(&core, "/harnesses");

    let ids: Vec<&str> = chips.iter().map(|c| c["id"].as_str().unwrap()).collect();
    assert_eq!(ids, ["claude", "codex"]);
    assert_eq!(chips[0]["name"], "Claude Code");
    assert_eq!(chips[1]["skill_count"], 2);
    assert_eq!(
        core.list_harnesses()
            .iter()
            .map(|h| h.id.as_str())
            .collect::<Vec<_>>(),
        ids
    );
}

#[test]
fn the_shared_agents_folder_is_its_own_chip() {
    let home = TempDir::new().unwrap();
    write_skill(&home.path().join(".agents/skills"), "graphify");

    let core = start(&home);
    let chips = get(&core, "/harnesses");
    let skills = get(&core, "/skills");

    assert_eq!(chips.len(), 1);
    assert_eq!(chips[0]["id"], "shared");
    assert_eq!(chips[0]["name"], "Shared");
    let graphify = skill(&skills, "shared:graphify");
    assert_eq!(graphify["harness"], "shared");
}

#[test]
fn claude_and_codex_skills_carry_their_typed_invocation_with_a_trailing_space() {
    let home = TempDir::new().unwrap();
    write_skill(&home.path().join(dir_of("claude")), "tdd");
    write_skill(
        &home
            .path()
            .join(".claude/plugins/cache/official/superpowers/1.0.0/skills"),
        "brainstorming",
    );
    write_skill(&home.path().join(dir_of("codex")), "review");

    let skills = get(&start(&home), "/skills");

    assert_eq!(skill(&skills, "claude:tdd")["invocation"], "/tdd ");
    assert_eq!(
        skill(&skills, "claude:superpowers:brainstorming")["invocation"],
        "/superpowers:brainstorming "
    );
    assert_eq!(skill(&skills, "codex:review")["invocation"], "$review ");
}

#[test]
fn harnesses_without_a_typed_form_use_the_plain_text_default() {
    let home = TempDir::new().unwrap();
    for h in REGISTRY {
        write_skill(&home.path().join(h.skill_dirs[0]), "graphify");
    }

    let skills = get(&start(&home), "/skills");
    let invocation = |harness: &str| {
        skill(&skills, &format!("{harness}:graphify"))["invocation"]
            .as_str()
            .unwrap()
            .to_string()
    };

    // Gemini CLI has no documented typed form; the Shared folder spans several harnesses.
    assert_eq!(invocation("gemini"), "Use the graphify skill ");
    assert_eq!(invocation("shared"), "Use the graphify skill ");
    // Verified typed forms for the rest.
    assert_eq!(invocation("cursor"), "/graphify ");
    assert_eq!(invocation("copilot"), "/graphify ");
    assert_eq!(invocation("opencode"), "@graphify ");
    for h in REGISTRY {
        let text = invocation(h.id);
        assert!(text.ends_with(' ') && !text.ends_with("  "), "{text:?}");
    }
}

#[test]
fn skills_are_grouped_in_registry_order() {
    let home = TempDir::new().unwrap();
    write_skill(&home.path().join(".agents/skills"), "a-shared");
    write_skill(&home.path().join(dir_of("codex")), "b-codex");
    write_skill(&home.path().join(dir_of("claude")), "z-claude");

    let skills = get(&start(&home), "/skills");

    let harnesses: Vec<&str> = skills
        .iter()
        .map(|s| s["harness"].as_str().unwrap())
        .collect();
    assert_eq!(harnesses, ["claude", "codex", "shared"]);
}

#[test]
fn verified_invocation_forms_record_their_doc_sources() {
    for h in REGISTRY {
        if h.template != skills_pad_core::harness::DEFAULT_TEMPLATE {
            assert!(
                !h.sources.is_empty(),
                "{} has a typed form but no doc source",
                h.id
            );
        }
    }
}
