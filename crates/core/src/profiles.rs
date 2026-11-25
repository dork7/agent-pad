//! Profiles: named snapshots of every Phone's View, switched in one step.
//!
//! A Profile maps device IDs to Views. Switching applies each saved View to the Phone with
//! that ID; entries for Phones that are no longer known are skipped, Phones the Profile does
//! not mention keep their current View, and chips that no longer exist are dropped.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::{harness, Settings, State};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Profile {
    pub name: String,
    /// View per device ID; `None` means everything.
    pub views: BTreeMap<String, Option<Vec<String>>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProfileError {
    UnknownProfile,
    EmptyName,
    /// Another Profile already has this name.
    NameTaken,
    Io(String),
}

impl std::fmt::Display for ProfileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownProfile => f.write_str("That profile doesn't exist."),
            Self::EmptyName => f.write_str("Give the profile a name."),
            Self::NameTaken => f.write_str("A profile with that name already exists."),
            Self::Io(e) => write!(f, "Could not save settings: {e}"),
        }
    }
}

impl std::error::Error for ProfileError {}

fn chip_exists(chip: &str, settings: &Settings) -> bool {
    if let Some(id) = chip.strip_prefix("h:") {
        harness::entries(settings).iter().any(|h| h.id == id)
    } else if let Some(id) = chip.strip_prefix("d:") {
        settings.directories.iter().any(|d| d.id == id)
    } else {
        false
    }
}

impl State {
    fn change_profiles<T>(
        &self,
        change: impl FnOnce(&mut Settings) -> Result<T, ProfileError>,
    ) -> Result<T, ProfileError> {
        let mut settings = self.settings.lock().unwrap();
        let mut updated = settings.clone();
        let value = change(&mut updated)?;
        crate::settings::save(&self.home, &updated).map_err(|e| ProfileError::Io(e.to_string()))?;
        *settings = updated;
        Ok(value)
    }

    pub(crate) fn save_profile(&self, name: &str) -> Result<(), ProfileError> {
        let name = name.trim();
        if name.is_empty() {
            return Err(ProfileError::EmptyName);
        }
        self.change_profiles(|s| {
            let views = s
                .phones
                .iter()
                .map(|p| (p.id.clone(), p.view.clone()))
                .collect();
            let profile = Profile {
                name: name.to_string(),
                views,
            };
            match s.profiles.iter_mut().find(|p| p.name == name) {
                Some(existing) => *existing = profile,
                None => s.profiles.push(profile),
            }
            s.active_profile = Some(name.to_string());
            Ok(())
        })
    }

    pub(crate) fn switch_profile(&self, name: &str) -> Result<(), ProfileError> {
        self.change_profiles(|s| {
            let profile = s
                .profiles
                .iter()
                .find(|p| p.name == name)
                .cloned()
                .ok_or(ProfileError::UnknownProfile)?;
            let snapshot = s.clone();
            for phone in &mut s.phones {
                if let Some(view) = profile.views.get(&phone.id) {
                    phone.view = view.as_ref().map(|chips| {
                        chips
                            .iter()
                            .filter(|c| chip_exists(c, &snapshot))
                            .cloned()
                            .collect()
                    });
                }
            }
            s.active_profile = Some(name.to_string());
            Ok(())
        })
    }

    pub(crate) fn rename_profile(&self, name: &str, new_name: &str) -> Result<(), ProfileError> {
        let new_name = new_name.trim();
        if new_name.is_empty() {
            return Err(ProfileError::EmptyName);
        }
        self.change_profiles(|s| {
            if new_name != name && s.profiles.iter().any(|p| p.name == new_name) {
                return Err(ProfileError::NameTaken);
            }
            let profile = s
                .profiles
                .iter_mut()
                .find(|p| p.name == name)
                .ok_or(ProfileError::UnknownProfile)?;
            profile.name = new_name.to_string();
            if s.active_profile.as_deref() == Some(name) {
                s.active_profile = Some(new_name.to_string());
            }
            Ok(())
        })
    }

    pub(crate) fn delete_profile(&self, name: &str) -> Result<(), ProfileError> {
        self.change_profiles(|s| {
            let before = s.profiles.len();
            s.profiles.retain(|p| p.name != name);
            if s.profiles.len() == before {
                return Err(ProfileError::UnknownProfile);
            }
            if s.active_profile.as_deref() == Some(name) {
                s.active_profile = None;
            }
            Ok(())
        })
    }
}
