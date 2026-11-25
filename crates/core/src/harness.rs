//! The registry of known Harnesses: where each keeps its global skills and how a skill is
//! typed into it (its Invocation template).
//!
//! A Harness is only shown when skills are found for it. `~/.agents/skills` is not owned by
//! any one Harness, so it is listed as its own "Shared" chip.

use serde::{Deserialize, Serialize};

use crate::Settings;

/// Placeholder in an Invocation template that is replaced by the skill name.
pub const NAME: &str = "{name}";

/// Template for Harnesses without a documented typed form.
pub const DEFAULT_TEMPLATE: &str = "Use the {name} skill ";

/// One entry in the registry.
#[derive(Debug, Clone, Copy)]
pub struct HarnessDef {
    /// Stable ID used in skill IDs and chips, e.g. `claude`.
    pub id: &'static str,
    /// Display name, e.g. `Claude Code`.
    pub name: &'static str,
    /// Global skill folders, relative to the home directory. Each holds `<skill>/SKILL.md`.
    pub skill_dirs: &'static [&'static str],
    /// Invocation template; `{name}` is replaced by the skill name. Always ends in a space.
    pub template: &'static str,
    /// Where the skill folders and typed form were verified.
    pub sources: &'static [&'static str],
}

impl HarnessDef {
    /// The text typed into this Harness to trigger `skill_name`.
    pub fn invocation(&self, skill_name: &str) -> String {
        render(self.template, skill_name)
    }
}

/// Fills a template, guaranteeing exactly one trailing space so the user can keep typing.
pub fn render(template: &str, skill_name: &str) -> String {
    let text = template.replace(NAME, skill_name);
    format!("{} ", text.trim_end())
}

/// A Harness chip as shown in the Pad: only Harnesses with at least one skill appear.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Harness {
    pub id: String,
    pub name: String,
    pub skill_count: usize,
}

/// ID of the Shared chip for `~/.agents/skills`.
pub const SHARED: &str = "shared";

/// Known Harnesses, in display order. Claude Code's plugin skills are scanned separately
/// (see `skills::scan_claude_plugins`) because of their versioned cache layout.
pub const REGISTRY: &[HarnessDef] = &[
    HarnessDef {
        id: "claude",
        name: "Claude Code",
        skill_dirs: &[".claude/skills"],
        template: "/{name} ",
        sources: &["https://code.claude.com/docs/en/skills"],
    },
    HarnessDef {
        id: "codex",
        name: "Codex",
        // Codex's documented global folder is `~/.agents/skills`, which is the Shared chip.
        // `~/.codex/skills` is not in the official docs but still loads per user reports.
        skill_dirs: &[".codex/skills"],
        template: "${name} ",
        sources: &["https://learn.chatgpt.com/docs/build-skills"],
    },
    HarnessDef {
        id: "cursor",
        name: "Cursor",
        skill_dirs: &[".cursor/skills"],
        template: "/{name} ",
        sources: &["https://cursor.com/docs/context/skills"],
    },
    HarnessDef {
        id: "gemini",
        name: "Gemini CLI",
        // No documented typed form: the model activates skills itself.
        skill_dirs: &[".gemini/skills"],
        template: DEFAULT_TEMPLATE,
        sources: &["https://geminicli.com/docs/cli/skills/"],
    },
    HarnessDef {
        id: "opencode",
        name: "OpenCode",
        skill_dirs: &[".config/opencode/skills"],
        template: "@{name} ",
        sources: &["https://opencode.ai/docs/skills/"],
    },
    HarnessDef {
        id: "copilot",
        name: "Copilot",
        skill_dirs: &[".copilot/skills"],
        template: "/{name} ",
        sources: &[
            "https://docs.github.com/en/copilot/how-tos/copilot-cli/customize-copilot/add-skills",
            "https://code.visualstudio.com/docs/copilot/customization/agent-skills",
        ],
    },
    HarnessDef {
        id: SHARED,
        name: "Shared",
        // Read by Codex, Cursor, Gemini CLI, OpenCode and Copilot, but not Claude Code, so
        // it gets the plain-text default that works in any of them.
        skill_dirs: &[".agents/skills"],
        template: DEFAULT_TEMPLATE,
        sources: &[],
    },
];

/// A Harness the user defined: a name and an Invocation template. Its skills come only from
/// Skill Directories assigned to it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CustomHarness {
    pub id: String,
    pub name: String,
    pub template: String,
}

/// A Harness as the scanner uses it: a registry entry or custom Harness, with the user's
/// template override applied.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub id: String,
    pub name: String,
    /// Built-in global skill folders, relative to home (empty for custom Harnesses).
    pub skill_dirs: Vec<&'static str>,
    pub template: String,
}

impl Entry {
    pub fn invocation(&self, skill_name: &str) -> String {
        render(&self.template, skill_name)
    }
}

/// Registry Harnesses (with template overrides) followed by custom ones.
pub fn entries(settings: &Settings) -> Vec<Entry> {
    let builtin = REGISTRY.iter().map(|def| Entry {
        id: def.id.into(),
        name: def.name.into(),
        skill_dirs: def.skill_dirs.to_vec(),
        template: settings
            .templates
            .get(def.id)
            .cloned()
            .unwrap_or_else(|| def.template.into()),
    });
    let custom = settings.custom_harnesses.iter().map(|h| Entry {
        id: h.id.clone(),
        name: h.name.clone(),
        skill_dirs: Vec::new(),
        template: h.template.clone(),
    });
    builtin.chain(custom).collect()
}

/// Whether `template` can be used: it must contain `{name}`.
pub fn valid_template(template: &str) -> bool {
    template.contains(NAME)
}

/// Looks up a registry entry by ID.
pub fn def(id: &str) -> Option<&'static HarnessDef> {
    REGISTRY.iter().find(|h| h.id == id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_template_renders_with_exactly_one_trailing_space() {
        for h in REGISTRY {
            let text = h.invocation("tdd");
            assert!(text.contains("tdd"), "{}: {text:?}", h.id);
            assert!(
                text.ends_with(' ') && !text.ends_with("  "),
                "{}: {text:?}",
                h.id
            );
        }
    }

    #[test]
    fn ids_are_unique() {
        let mut ids: Vec<_> = REGISTRY.iter().map(|h| h.id).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), REGISTRY.len());
    }
}
