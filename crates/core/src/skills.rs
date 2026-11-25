//! Skill discovery. Skills are read from disk on every call and never cached.

use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::harness::{self, Entry, Harness};
use crate::Settings;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Skill {
    /// Stable ID the phone sends back, e.g. `claude:tdd` or `claude:superpowers:brainstorming`.
    pub id: String,
    /// Harness (or the Shared chip) the skill belongs to, e.g. `claude`, `codex` or `shared`.
    pub harness: String,
    /// Skill name as typed in the harness, e.g. `tdd` or `superpowers:brainstorming`.
    pub name: String,
    /// First line of the description from SKILL.md, or empty.
    pub description: String,
    /// Text pasted to trigger the skill in its harness, e.g. `/tdd ` or `$tdd `.
    /// Always ends in a single space.
    pub invocation: String,
    /// ID of the added Skill Directory it was found in, or `None` for built-in locations.
    pub directory: Option<String>,
    /// Saved argument presets, each a one-tap button on the phone.
    pub presets: Vec<crate::saved_text::PresetRef>,
}

/// A Skill Directory the user added, assigned to a Harness.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AddedDirectory {
    pub id: String,
    /// Absolute path of the folder holding `<skill>/SKILL.md`.
    pub path: PathBuf,
    pub harness: String,
}

/// A Skill Directory chip as shown in the Pad.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Directory {
    pub id: String,
    pub path: PathBuf,
    pub harness: String,
    pub skill_count: usize,
}

/// All skills found under `home` and in added directories, sorted by harness (registry order,
/// then custom harnesses) then name. A skill found twice under the same ID appears once,
/// built-in locations first.
pub fn scan(home: &Path, settings: &Settings) -> Vec<Skill> {
    let entries = harness::entries(settings);
    let mut skills = Vec::new();
    for entry in &entries {
        for dir in &entry.skill_dirs {
            for (dir_name, skill_md) in skill_files(&home.join(dir)) {
                skills.push(make_skill(entry, None, None, &dir_name, &skill_md));
            }
        }
        if entry.id == "claude" {
            skills.extend(scan_claude_plugins(home, entry));
        }
        for added in settings
            .directories
            .iter()
            .filter(|d| d.harness == entry.id)
        {
            for (dir_name, skill_md) in skill_files(&added.path) {
                skills.push(make_skill(
                    entry,
                    Some(&added.id),
                    None,
                    &dir_name,
                    &skill_md,
                ));
            }
        }
    }
    let order = |id: &str| entries.iter().position(|h| h.id == id);
    // Stable sort keeps built-in locations ahead of added ones for the same ID.
    skills.sort_by(|a, b| (order(&a.harness), &a.name).cmp(&(order(&b.harness), &b.name)));
    skills.dedup_by(|a, b| a.id == b.id);
    for skill in &mut skills {
        skill.presets = settings
            .presets
            .iter()
            .filter(|p| p.skill == skill.id)
            .map(|p| crate::saved_text::PresetRef {
                id: p.id.clone(),
                text: p.text.clone(),
            })
            .collect();
    }
    skills
}

/// Harness chips: one per harness with at least one skill, in scan order.
pub fn harnesses(skills: &[Skill], settings: &Settings) -> Vec<Harness> {
    harness::entries(settings)
        .into_iter()
        .filter_map(|entry| {
            let skill_count = skills.iter().filter(|s| s.harness == entry.id).count();
            (skill_count > 0).then_some(Harness {
                id: entry.id,
                name: entry.name,
                skill_count,
            })
        })
        .collect()
}

/// Added Skill Directory chips with their skill counts.
pub fn directories(skills: &[Skill], settings: &Settings) -> Vec<Directory> {
    settings
        .directories
        .iter()
        .map(|d| Directory {
            id: d.id.clone(),
            path: d.path.clone(),
            harness: d.harness.clone(),
            skill_count: skills
                .iter()
                .filter(|s| s.directory.as_deref() == Some(d.id.as_str()))
                .count(),
        })
        .collect()
}

/// Claude Code plugin skills in
/// `~/.claude/plugins/cache/<marketplace>/<plugin>/<version>/skills/<skill>/SKILL.md`,
/// where only the newest version of each plugin counts. Named `<plugin>:<skill>`.
fn scan_claude_plugins(home: &Path, entry: &Entry) -> Vec<Skill> {
    let mut skills = Vec::new();
    for marketplace in subdirs(&home.join(".claude/plugins/cache")) {
        for plugin in subdirs(&marketplace) {
            let Some(newest) = newest_version(&plugin) else {
                continue;
            };
            let plugin_name = file_name(&plugin);
            for (dir_name, skill_md) in skill_files(&newest.join("skills")) {
                skills.push(make_skill(
                    entry,
                    None,
                    Some(&plugin_name),
                    &dir_name,
                    &skill_md,
                ));
            }
        }
    }
    skills
}

fn make_skill(
    entry: &Entry,
    directory: Option<&str>,
    plugin: Option<&str>,
    dir_name: &str,
    skill_md: &Path,
) -> Skill {
    let front = fs::read_to_string(skill_md)
        .map(|t| parse_frontmatter(&t))
        .unwrap_or_default();
    let base = front.name.unwrap_or_else(|| dir_name.to_string());
    let name = match plugin {
        Some(plugin) => format!("{plugin}:{base}"),
        None => base,
    };
    Skill {
        id: format!("{}:{name}", entry.id),
        harness: entry.id.clone(),
        invocation: entry.invocation(&name),
        name,
        description: front.description.unwrap_or_default(),
        directory: directory.map(str::to_string),
        presets: Vec::new(),
    }
}

/// `(directory name, path to SKILL.md)` for each `<dir>/<skill>/SKILL.md` that exists.
fn skill_files(dir: &Path) -> Vec<(String, PathBuf)> {
    subdirs(dir)
        .into_iter()
        .map(|d| (file_name(&d), d.join("SKILL.md")))
        .filter(|(_, md)| md.is_file())
        .collect()
}

fn subdirs(dir: &Path) -> Vec<PathBuf> {
    let Ok(entries) = fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut dirs: Vec<PathBuf> = entries
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.is_dir() && !file_name(p).starts_with('.'))
        .collect();
    dirs.sort();
    dirs
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default()
}

fn newest_version(plugin: &Path) -> Option<PathBuf> {
    subdirs(plugin)
        .into_iter()
        .max_by(|a, b| compare_versions(&file_name(a), &file_name(b)))
}

/// Compares version strings part by part, numerically where both parts are numbers,
/// so `4.10.0` is newer than `4.9.2`.
fn compare_versions(a: &str, b: &str) -> Ordering {
    let parts = |v: &str| {
        v.split(['.', '-', '+'])
            .map(str::to_string)
            .collect::<Vec<_>>()
    };
    let (pa, pb) = (parts(a), parts(b));
    for (x, y) in pa.iter().zip(&pb) {
        let ord = match (x.parse::<u64>(), y.parse::<u64>()) {
            (Ok(x), Ok(y)) => x.cmp(&y),
            _ => x.cmp(y),
        };
        if ord != Ordering::Equal {
            return ord;
        }
    }
    pa.len().cmp(&pb.len())
}

#[derive(Default)]
struct Frontmatter {
    name: Option<String>,
    description: Option<String>,
}

/// Reads `name` and `description` from YAML frontmatter. Handles plain, quoted and
/// block (`|`, `>`) values, which covers how SKILL.md files are written in practice.
/// The description is reduced to its first line.
fn parse_frontmatter(text: &str) -> Frontmatter {
    let mut lines = text.lines();
    if lines.next().map(str::trim) != Some("---") {
        return Frontmatter::default();
    }
    let body: Vec<&str> = lines.take_while(|l| l.trim() != "---").collect();

    let mut fields = BTreeMap::new();
    let mut i = 0;
    while i < body.len() {
        let line = body[i];
        i += 1;
        if line.starts_with([' ', '\t']) {
            continue;
        }
        let Some((key, value)) = line.split_once(':') else {
            continue;
        };
        let value = value.trim();
        let value = if let Some(style) = value.chars().next().filter(|c| *c == '|' || *c == '>') {
            let mut block = Vec::new();
            while i < body.len() && (body[i].starts_with([' ', '\t']) || body[i].trim().is_empty())
            {
                block.push(body[i].trim());
                i += 1;
            }
            let sep = if style == '>' { " " } else { "\n" };
            block.join(sep).trim().to_string()
        } else {
            unquote(value)
        };
        fields.insert(key.trim().to_string(), value);
    }

    let first_line = |s: String| s.lines().next().unwrap_or("").trim().to_string();
    Frontmatter {
        name: fields.remove("name").filter(|s| !s.is_empty()),
        description: fields
            .remove("description")
            .map(first_line)
            .filter(|s| !s.is_empty()),
    }
}

fn unquote(value: &str) -> String {
    let v = value.trim();
    if v.len() >= 2 && v.starts_with('"') && v.ends_with('"') {
        v[1..v.len() - 1].replace("\\\"", "\"")
    } else if v.len() >= 2 && v.starts_with('\'') && v.ends_with('\'') {
        v[1..v.len() - 1].replace("''", "'")
    } else {
        v.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn versions_compare_numerically() {
        assert_eq!(compare_versions("4.10.0", "4.9.2"), Ordering::Greater);
        assert_eq!(compare_versions("1.0.0", "1.0.0"), Ordering::Equal);
        assert_eq!(compare_versions("2.0", "10.0"), Ordering::Less);
    }

    #[test]
    fn frontmatter_handles_quoted_and_block_values() {
        let f =
            parse_frontmatter("---\nname: 'it''s'\ndescription: >\n  Folded\n  text.\n---\nbody");
        assert_eq!(f.name.as_deref(), Some("it's"));
        assert_eq!(f.description.as_deref(), Some("Folded text."));

        let f = parse_frontmatter("---\ndescription: |\n  First line.\n  Second line.\n---");
        assert_eq!(f.description.as_deref(), Some("First line."));

        let f = parse_frontmatter("---\ndescription: \"Uses: colons\"\n---");
        assert_eq!(f.description.as_deref(), Some("Uses: colons"));
    }

    #[test]
    fn no_frontmatter_yields_nothing() {
        let f = parse_frontmatter("# Just a heading");
        assert!(f.name.is_none() && f.description.is_none());
    }
}
