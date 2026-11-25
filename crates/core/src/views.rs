//! Views: which skills each Phone may see.
//!
//! A View is either everything (`None`, the default for new Phones, which also covers
//! harnesses and directories added later) or a list of chips: `h:<harness id>` gives all of
//! a harness's skills, including added directories assigned to it; `d:<directory id>` gives
//! only that added directory's skills. An empty list shows nothing.

use crate::harness::Harness;
use crate::phones::PhoneError;
use crate::skills::{self, Skill};
use crate::{harness, PasteError, Settings, State};

/// Chip for a harness.
pub fn harness_chip(id: &str) -> String {
    format!("h:{id}")
}

/// Chip for an added Skill Directory.
pub fn directory_chip(id: &str) -> String {
    format!("d:{id}")
}

/// Whether a Phone with `view` may see `skill`.
pub(crate) fn sees(view: Option<&[String]>, skill: &Skill) -> bool {
    let Some(view) = view else {
        return true;
    };
    view.iter().any(|chip| {
        chip.strip_prefix("h:") == Some(skill.harness.as_str())
            || (chip.strip_prefix("d:").is_some()
                && chip.strip_prefix("d:") == skill.directory.as_deref())
    })
}

/// Every chip that currently exists: harnesses with skills, then added directories.
fn all_chips(skills: &[Skill], settings: &Settings) -> Vec<String> {
    let harnesses = skills::harnesses(skills, settings)
        .into_iter()
        .map(|h| harness_chip(&h.id));
    let directories = settings.directories.iter().map(|d| directory_chip(&d.id));
    harnesses.chain(directories).collect()
}

fn known_chip(chip: &str, settings: &Settings) -> bool {
    if let Some(id) = chip.strip_prefix("h:") {
        harness::entries(settings).iter().any(|h| h.id == id)
    } else if let Some(id) = chip.strip_prefix("d:") {
        settings.directories.iter().any(|d| d.id == id)
    } else {
        false
    }
}

impl State {
    fn view_of(settings: &Settings, device_id: &str) -> Option<Vec<String>> {
        settings
            .phones
            .iter()
            .find(|p| p.id == device_id)
            .and_then(|p| p.view.clone())
    }

    /// The Phone's View as sent to the phone page, so it can notice changes and reload.
    pub(crate) fn view_for_phone(&self, device_id: &str) -> Option<Vec<String>> {
        Self::view_of(&self.settings.lock().unwrap(), device_id)
    }

    /// Skills this Phone may see, freshly scanned.
    pub(crate) fn skills_for_phone(&self, device_id: &str) -> Vec<Skill> {
        let settings = self.settings.lock().unwrap().clone();
        let view = Self::view_of(&settings, device_id);
        skills::scan(&self.home, &settings)
            .into_iter()
            .filter(|s| sees(view.as_deref(), s))
            .collect()
    }

    /// Harness tabs for this Phone: harnesses with at least one visible skill.
    pub(crate) fn harnesses_for_phone(&self, device_id: &str) -> Vec<Harness> {
        let settings = self.settings.lock().unwrap().clone();
        skills::harnesses(&self.skills_for_phone(device_id), &settings)
    }

    /// Pastes a skill for this Phone, refusing skills outside its View.
    pub(crate) fn paste_for_phone(
        &self,
        device_id: &str,
        skill_id: &str,
    ) -> Result<Skill, PasteError> {
        let view = self.view_for_phone(device_id);
        let skill = self
            .scan()
            .into_iter()
            .find(|s| s.id == skill_id)
            .ok_or(PasteError::UnknownSkill)?;
        if !sees(view.as_deref(), &skill) {
            return Err(PasteError::NotInView);
        }
        self.paste_text(skill.invocation.clone());
        Ok(skill)
    }

    /// Replaces a Phone's View. Every chip must exist.
    pub(crate) fn set_view(
        &self,
        device_id: &str,
        view: Option<Vec<String>>,
    ) -> Result<(), PhoneError> {
        {
            let settings = self.settings.lock().unwrap();
            let unknown = view
                .iter()
                .flatten()
                .any(|chip| !known_chip(chip, &settings));
            if unknown {
                return Err(PhoneError::UnknownChip);
            }
        }
        self.update_phone(device_id, |p| p.view = view.map(dedup))
    }

    /// Adds a chip to a Phone's View. A Phone that sees everything is unchanged.
    pub(crate) fn add_chip(&self, device_id: &str, chip: &str) -> Result<(), PhoneError> {
        if !known_chip(chip, &self.settings.lock().unwrap()) {
            return Err(PhoneError::UnknownChip);
        }
        self.update_phone(device_id, |p| {
            if let Some(view) = &mut p.view {
                if !view.iter().any(|c| c == chip) {
                    view.push(chip.to_string());
                }
            }
        })
    }

    /// Removes a chip. A Phone that saw everything now sees every current chip but this one.
    pub(crate) fn remove_chip(&self, device_id: &str, chip: &str) -> Result<(), PhoneError> {
        let all = {
            let settings = self.settings.lock().unwrap().clone();
            all_chips(&skills::scan(&self.home, &settings), &settings)
        };
        self.update_phone(device_id, |p| {
            let view = p.view.get_or_insert(all);
            view.retain(|c| c != chip);
        })
    }

    /// Every chip that currently exists, for the Pad's palette.
    pub(crate) fn chips(&self) -> Vec<String> {
        let settings = self.settings.lock().unwrap().clone();
        all_chips(&skills::scan(&self.home, &settings), &settings)
    }
}

fn dedup(chips: Vec<String>) -> Vec<String> {
    let mut seen = Vec::new();
    for chip in chips {
        if !seen.contains(&chip) {
            seen.push(chip);
        }
    }
    seen
}

#[cfg(test)]
mod tests {
    use super::*;

    fn skill(harness: &str, directory: Option<&str>) -> Skill {
        Skill {
            id: format!("{harness}:x"),
            harness: harness.into(),
            name: "x".into(),
            description: String::new(),
            invocation: "x ".into(),
            directory: directory.map(Into::into),
        }
    }

    #[test]
    fn chips_select_skills() {
        let claude = skill("claude", None);
        let team = skill("claude", Some("team"));
        let codex = skill("codex", None);
        let view = |chips: &[&str]| chips.iter().map(|c| c.to_string()).collect::<Vec<_>>();

        assert!(sees(None, &codex));
        let only_team = view(&["d:team"]);
        assert!(sees(Some(&only_team), &team));
        assert!(!sees(Some(&only_team), &claude));
        let claude_all = view(&["h:claude"]);
        assert!(
            sees(Some(&claude_all), &team),
            "a harness chip includes its added directories"
        );
        assert!(!sees(Some(&claude_all), &codex));
        assert!(!sees(Some(&[]), &claude));
    }
}
