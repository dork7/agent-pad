//! Text the user saves in the Pad and pastes from the phone by ID: argument presets on skills
//! (ticket 16) and snippets (ticket 17). The phone never sends the text itself.

use serde::{Deserialize, Serialize};

use crate::settings::SettingsError;
use crate::views;
use crate::{Core, PasteError, State};

/// A saved suffix for one skill, e.g. `high` for a review skill.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Preset {
    pub id: String,
    /// The skill it belongs to, by `<harness>:<name>`, so it survives rescans.
    pub skill: String,
    pub text: String,
}

/// A preset as listed with its skill on the phone.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PresetRef {
    pub id: String,
    pub text: String,
}

/// Saved non-skill text pasted from the Snippets tab.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Snippet {
    pub id: String,
    pub title: String,
    pub text: String,
}

/// The View chip that gives a Phone the Snippets tab.
pub const SNIPPETS_CHIP: &str = "snippets";

fn new_id() -> String {
    let mut bytes = [0; 8];
    getrandom::fill(&mut bytes).expect("system randomness");
    format!("{:016x}", u64::from_be_bytes(bytes))
}

impl State {
    /// Pastes a skill with one of its presets for this Phone: the Invocation, the preset, then a
    /// trailing space.
    pub(crate) fn paste_preset_for_phone(
        &self,
        device_id: &str,
        skill_id: &str,
        preset_id: &str,
    ) -> Result<(), PasteError> {
        let skill = self
            .skills_for_phone_unfiltered(skill_id)
            .ok_or(PasteError::UnknownSkill)?;
        if !views::sees(self.view_for_phone(device_id).as_deref(), &skill) {
            return Err(PasteError::NotInView);
        }
        let preset = skill
            .presets
            .iter()
            .find(|p| p.id == preset_id)
            .ok_or(PasteError::UnknownPreset)?;
        self.paste_text(format!(
            "{} {} ",
            skill.invocation.trim_end(),
            preset.text.trim()
        ));
        Ok(())
    }

    fn skills_for_phone_unfiltered(&self, skill_id: &str) -> Option<crate::Skill> {
        self.scan().into_iter().find(|s| s.id == skill_id)
    }

    /// Whether this Phone's View includes the Snippets chip.
    fn sees_snippets(&self, device_id: &str) -> bool {
        self.view_for_phone(device_id)
            .is_none_or(|view| view.iter().any(|c| c == SNIPPETS_CHIP))
    }

    /// Snippets for this Phone: all of them if its View includes the Snippets chip, else none.
    pub(crate) fn snippets_for_phone(&self, device_id: &str) -> Vec<Snippet> {
        if !self.sees_snippets(device_id) {
            return Vec::new();
        }
        self.settings.lock().unwrap().snippets.clone()
    }

    /// Pastes a snippet's text exactly as saved.
    pub(crate) fn paste_snippet_for_phone(
        &self,
        device_id: &str,
        snippet_id: &str,
    ) -> Result<(), PasteError> {
        let text = self
            .settings
            .lock()
            .unwrap()
            .snippets
            .iter()
            .find(|s| s.id == snippet_id)
            .map(|s| s.text.clone())
            .ok_or(PasteError::UnknownSnippet)?;
        if !self.sees_snippets(device_id) {
            return Err(PasteError::NotInView);
        }
        self.paste_text(text);
        Ok(())
    }
}

impl Core {
    /// Presets of one skill (`<harness>:<name>`), in the order they were added.
    pub fn list_presets(&self, skill_id: &str) -> Vec<Preset> {
        let settings = self.state.settings.lock().unwrap();
        settings
            .presets
            .iter()
            .filter(|p| p.skill == skill_id)
            .cloned()
            .collect()
    }

    /// Adds a preset to a skill. Returns its ID.
    pub fn add_preset(&self, skill_id: &str, text: &str) -> Result<String, SettingsError> {
        let text = text.trim();
        if text.is_empty() {
            return Err(SettingsError::EmptyText);
        }
        self.state.change_settings(|s| {
            let id = new_id();
            s.presets.push(Preset {
                id: id.clone(),
                skill: skill_id.to_string(),
                text: text.to_string(),
            });
            Ok(id)
        })
    }

    pub fn update_preset(&self, id: &str, text: &str) -> Result<(), SettingsError> {
        let text = text.trim();
        if text.is_empty() {
            return Err(SettingsError::EmptyText);
        }
        self.state.change_settings(|s| {
            let preset = s
                .presets
                .iter_mut()
                .find(|p| p.id == id)
                .ok_or(SettingsError::UnknownItem)?;
            preset.text = text.to_string();
            Ok(())
        })
    }

    pub fn delete_preset(&self, id: &str) -> Result<(), SettingsError> {
        self.state.change_settings(|s| {
            let before = s.presets.len();
            s.presets.retain(|p| p.id != id);
            if s.presets.len() == before {
                return Err(SettingsError::UnknownItem);
            }
            Ok(())
        })
    }

    /// Snippets in their saved order.
    pub fn list_snippets(&self) -> Vec<Snippet> {
        self.state.settings.lock().unwrap().snippets.clone()
    }

    /// Adds a snippet at the end. Its text is pasted exactly as given.
    pub fn add_snippet(&self, title: &str, text: &str) -> Result<String, SettingsError> {
        if text.is_empty() {
            return Err(SettingsError::EmptyText);
        }
        self.state.change_settings(|s| {
            let id = new_id();
            s.snippets.push(Snippet {
                id: id.clone(),
                title: title.trim().to_string(),
                text: text.to_string(),
            });
            Ok(id)
        })
    }

    pub fn update_snippet(&self, id: &str, title: &str, text: &str) -> Result<(), SettingsError> {
        if text.is_empty() {
            return Err(SettingsError::EmptyText);
        }
        self.state.change_settings(|s| {
            let snippet = s
                .snippets
                .iter_mut()
                .find(|x| x.id == id)
                .ok_or(SettingsError::UnknownItem)?;
            snippet.title = title.trim().to_string();
            snippet.text = text.to_string();
            Ok(())
        })
    }

    /// Moves a snippet to `index` (clamped to the end).
    pub fn move_snippet(&self, id: &str, index: usize) -> Result<(), SettingsError> {
        self.state.change_settings(|s| {
            let from = s
                .snippets
                .iter()
                .position(|x| x.id == id)
                .ok_or(SettingsError::UnknownItem)?;
            let snippet = s.snippets.remove(from);
            let to = index.min(s.snippets.len());
            s.snippets.insert(to, snippet);
            Ok(())
        })
    }

    pub fn delete_snippet(&self, id: &str) -> Result<(), SettingsError> {
        self.state.change_settings(|s| {
            let before = s.snippets.len();
            s.snippets.retain(|x| x.id != id);
            if s.snippets.len() == before {
                return Err(SettingsError::UnknownItem);
            }
            Ok(())
        })
    }
}
