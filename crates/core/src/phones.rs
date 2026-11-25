//! Paired Phones: who they are, whether they are online, and whether they are let in.
//!
//! Each Phone gets a long-lived device ID cookie on its first authenticated request, so it is
//! recognized across sharing sessions (new token, same device ID). Known Phones are saved in
//! `settings.json`. Cookies are scoped to the computer's address, not the port, so they survive
//! the random port of each sharing session; a new LAN address makes a Phone look new.
//!
//! Disconnect and Forget both shut a Phone out until it scans the QR code again (a request that
//! carries the token in its URL). Forget also deletes its saved record and View.

use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::{settings, State};

/// A Phone counts as online if it made a request this recently (two missed 30 s heartbeats).
const ONLINE_WINDOW: Duration = Duration::from_secs(75);

/// `last_seen` is saved at most this often, so heartbeats don't rewrite the file every time.
const LAST_SEEN_SAVE_EVERY: u64 = 60;

/// A known Phone as saved in settings.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PhoneRecord {
    pub id: String,
    pub name: String,
    /// Unix seconds of the last request.
    pub last_seen: u64,
    /// Chips this Phone may see (`h:<harness>` or `d:<directory>`); `None` means everything.
    #[serde(default)]
    pub view: Option<Vec<String>>,
}

/// A Phone as shown in the Pad.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Phone {
    pub id: String,
    pub name: String,
    pub online: bool,
    /// Unix seconds of the last request.
    pub last_seen: u64,
    /// Chips this Phone may see; `None` means everything.
    pub view: Option<Vec<String>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PhoneError {
    /// No known Phone has this ID.
    UnknownPhone,
}

/// In-memory Phone state that is not saved.
#[derive(Default)]
pub(crate) struct Runtime {
    last_activity: HashMap<String, Instant>,
    /// Disconnected or forgotten during this run; let in again only by scanning.
    shut_out: HashSet<String>,
}

/// Outcome of an authenticated request from a Phone.
pub(crate) enum Admission {
    /// Let in. `new_cookie` is set when the Phone needs a (new) device ID cookie.
    Admitted { device_id: String, new_cookie: bool },
    /// Disconnected or forgotten and has not scanned again.
    ShutOut,
}

impl State {
    /// Admits a request that already passed the token check, registering new Phones.
    pub(crate) fn admit_phone(
        &self,
        device_cookie: Option<&str>,
        scanned: bool,
        user_agent: &str,
    ) -> Admission {
        let now = self.clock.now();
        let mut runtime = self.phones.lock().unwrap();
        if let Some(id) = device_cookie {
            if runtime.shut_out.contains(id) {
                if !scanned {
                    return Admission::ShutOut;
                }
                runtime.shut_out.remove(id);
            }
        }

        let mut settings = self.settings.lock().unwrap();
        let unix = unix_now();
        let (device_id, new_cookie) = match device_cookie
            .filter(|id| valid_id(id) && settings.phones.iter().any(|p| p.id == *id))
        {
            Some(id) => {
                let record = settings.phones.iter_mut().find(|p| p.id == id).unwrap();
                let stale = unix.saturating_sub(record.last_seen) >= LAST_SEEN_SAVE_EVERY;
                record.last_seen = unix;
                if stale {
                    let _ = settings::save(&self.home, &settings);
                }
                (id.to_string(), false)
            }
            None => {
                // New, forgotten earlier, or a cookie we can't use: register a fresh Phone.
                let id = new_id();
                let name = unique_name(&settings.phones, &auto_name(user_agent));
                settings.phones.push(PhoneRecord {
                    id: id.clone(),
                    name,
                    last_seen: unix,
                    view: None,
                });
                let _ = settings::save(&self.home, &settings);
                (id, true)
            }
        };
        runtime.last_activity.insert(device_id.clone(), now);
        Admission::Admitted {
            device_id,
            new_cookie,
        }
    }

    pub(crate) fn list_phones(&self) -> Vec<Phone> {
        let now = self.clock.now();
        let runtime = self.phones.lock().unwrap();
        let settings = self.settings.lock().unwrap();
        settings
            .phones
            .iter()
            .map(|p| Phone {
                id: p.id.clone(),
                name: p.name.clone(),
                online: !runtime.shut_out.contains(&p.id)
                    && runtime
                        .last_activity
                        .get(&p.id)
                        .is_some_and(|t| now.duration_since(*t) < ONLINE_WINDOW),
                last_seen: p.last_seen,
                view: p.view.clone(),
            })
            .collect()
    }

    /// Applies `change` to one saved Phone and saves settings.
    pub(crate) fn update_phone(
        &self,
        id: &str,
        change: impl FnOnce(&mut PhoneRecord),
    ) -> Result<(), PhoneError> {
        let mut settings = self.settings.lock().unwrap();
        let record = settings
            .phones
            .iter_mut()
            .find(|p| p.id == id)
            .ok_or(PhoneError::UnknownPhone)?;
        change(record);
        let _ = settings::save(&self.home, &settings);
        Ok(())
    }

    pub(crate) fn disconnect_phone(&self, id: &str) -> Result<(), PhoneError> {
        let known = self
            .settings
            .lock()
            .unwrap()
            .phones
            .iter()
            .any(|p| p.id == id);
        if !known {
            return Err(PhoneError::UnknownPhone);
        }
        let mut runtime = self.phones.lock().unwrap();
        runtime.shut_out.insert(id.to_string());
        runtime.last_activity.remove(id);
        Ok(())
    }

    pub(crate) fn forget_phone(&self, id: &str) -> Result<(), PhoneError> {
        {
            let mut settings = self.settings.lock().unwrap();
            let before = settings.phones.len();
            settings.phones.retain(|p| p.id != id);
            if settings.phones.len() == before {
                return Err(PhoneError::UnknownPhone);
            }
            let _ = settings::save(&self.home, &settings);
        }
        let mut runtime = self.phones.lock().unwrap();
        runtime.shut_out.insert(id.to_string());
        runtime.last_activity.remove(id);
        Ok(())
    }
}

fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn new_id() -> String {
    let mut bytes = [0; 16];
    getrandom::fill(&mut bytes).expect("system randomness");
    format!("{:032x}", u128::from_be_bytes(bytes))
}

/// Device IDs we mint are 32 lowercase hex characters; anything else is ignored.
fn valid_id(id: &str) -> bool {
    id.len() == 32 && id.bytes().all(|b| b.is_ascii_hexdigit())
}

/// A readable default name from the browser's User-Agent.
pub(crate) fn auto_name(user_agent: &str) -> String {
    let ua = user_agent;
    let name = if ua.contains("iPhone") {
        "iPhone"
    } else if ua.contains("iPad") {
        "iPad"
    } else if ua.contains("Android") {
        // Android UAs usually carry the model: "...; Android 14; SM-S918B) ..."
        if let Some(model) = android_model(ua) {
            return model;
        }
        "Android phone"
    } else if ua.contains("Macintosh") {
        "Mac"
    } else if ua.contains("Windows") {
        "Windows PC"
    } else {
        "Phone"
    };
    name.to_string()
}

fn android_model(ua: &str) -> Option<String> {
    let inside = ua.split_once('(')?.1.split_once(')')?.0;
    let model = inside
        .split(';')
        .map(str::trim)
        .skip_while(|part| !part.starts_with("Android"))
        .nth(1)?
        .split(" Build/")
        .next()?
        .trim();
    // Chrome's reduced UA sends "K" instead of the model.
    (!model.is_empty() && model != "K").then(|| model.to_string())
}

/// `base`, or `base 2`, `base 3`… if that name is taken.
fn unique_name(phones: &[PhoneRecord], base: &str) -> String {
    let taken = |name: &str| phones.iter().any(|p| p.name == name);
    if !taken(base) {
        return base.to_string();
    }
    (2..)
        .map(|n| format!("{base} {n}"))
        .find(|name| !taken(name))
        .expect("some name is free")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_come_from_the_user_agent() {
        let iphone = "Mozilla/5.0 (iPhone; CPU iPhone OS 18_0 like Mac OS X) AppleWebKit/605.1.15";
        let samsung = "Mozilla/5.0 (Linux; Android 14; SM-S918B Build/UP1A) AppleWebKit/537.36";
        let reduced = "Mozilla/5.0 (Linux; Android 10; K) AppleWebKit/537.36 Chrome/130 Mobile";
        assert_eq!(auto_name(iphone), "iPhone");
        assert_eq!(auto_name(samsung), "SM-S918B");
        assert_eq!(auto_name(reduced), "Android phone");
        assert_eq!(auto_name(""), "Phone");
    }

    #[test]
    fn duplicate_names_get_a_number() {
        let rec = |name: &str| PhoneRecord {
            id: name.into(),
            name: name.into(),
            last_seen: 0,
            view: None,
        };
        assert_eq!(unique_name(&[], "iPhone"), "iPhone");
        assert_eq!(unique_name(&[rec("iPhone")], "iPhone"), "iPhone 2");
        assert_eq!(
            unique_name(&[rec("iPhone"), rec("iPhone 2")], "iPhone"),
            "iPhone 3"
        );
    }

    #[test]
    fn only_minted_ids_are_accepted() {
        assert!(valid_id("0123456789abcdef0123456789abcdef"));
        assert!(!valid_id("guess"));
        assert!(!valid_id("../../etc/passwd................"));
    }
}
