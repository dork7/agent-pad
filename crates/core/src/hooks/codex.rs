//! Codex command hooks are configured inline in the user config.
//! https://developers.openai.com/codex/config-reference
//! https://developers.openai.com/codex/hooks

use toml_edit::{value, Array, ArrayOfTables, DocumentMut, Item, Table, TableLike, Value};

use super::{read_optional, settings_target, shell_quote, write_atomic, HookEvent, Receipt};
use crate::{sessions::Event, Core};
use serde::{Deserialize, Serialize};
use std::{fs, io, path::PathBuf};

const START_MATCHER: &str = "^startup$|^resume$|^clear$|^fork$";
const EVENTS: &[(&str, Option<&str>)] = &[
    ("SessionStart", Some(START_MATCHER)),
    ("UserPromptSubmit", None),
    ("PreToolUse", None),
    ("PostToolUse", None),
    ("PermissionRequest", None),
    ("Stop", None),
    ("Interrupt", None),
    ("SessionEnd", None),
];

/// Display after installing Codex hooks.
pub const CODEX_HOOKS_RESTART_NOTICE: &str =
    "Review and trust the new hooks in Codex (/hooks in the CLI), then start or resume a session.";

pub(super) fn event(payload: &HookEvent) -> Option<Event> {
    match payload.hook_event_name.as_str() {
        "SessionStart" if payload.source.as_deref() != Some("compact") => Some(Event::Start),
        "UserPromptSubmit" | "PostToolUse" => Some(Event::Working),
        "PreToolUse" => Some(Event::ToolStarted),
        "PermissionRequest" => Some(Event::Permission),
        "Stop" | "Interrupt" => Some(Event::Stop),
        "SessionEnd" => Some(Event::End),
        _ => None,
    }
}

/// Exact Codex configuration text to display before asking for consent. A missing
/// file has `before: None`. Pass the complete preview back to apply it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CodexHooksPreview {
    pub path: PathBuf,
    pub before: Option<String>,
    pub after: String,
    install_id: String,
}

impl Core {
    /// Compute the exact install change without writing files.
    pub fn preview_codex_hooks(&self) -> io::Result<CodexHooksPreview> {
        self.codex_hooks_preview(self.codex_hook_listener.install_id()?)
    }

    fn codex_config_path(&self) -> io::Result<PathBuf> {
        // An injected home uses its own config, independently of the host environment.
        let home = &self.state.home;
        let override_home = (std::env::home_dir().as_deref() == Some(home.as_path()))
            .then(|| std::env::var_os("CODEX_HOME"))
            .flatten()
            .filter(|path| !path.is_empty());
        let codex_home = override_home.map_or_else(|| home.join(".codex"), PathBuf::from);
        std::path::absolute(codex_home.join("config.toml"))
    }

    fn codex_hook_command(&self, install_id: &str) -> io::Result<String> {
        // Codex runs hooks through the configured shell. Windows uses the native
        // PowerShell command; other platforms keep the POSIX wrapper.
        // https://github.com/openai/codex/blob/main/codex-rs/hooks/src/engine/command_runner.rs
        let command = self.codex_hook_listener.command(install_id)?;
        if cfg!(windows) {
            return Ok(command);
        }
        Ok(format!("/bin/sh -c {}", shell_quote(&command)))
    }

    fn codex_hooks_preview(&self, install_id: String) -> io::Result<CodexHooksPreview> {
        let path = self.codex_config_path()?;
        let before = read_optional(&settings_target(&path)?)?;
        let command = self.codex_hook_command(&install_id)?;
        let mut config = parse(before.as_deref())?;
        let after = if add_hooks(&mut config, &command)? {
            render(&config, before.as_deref().unwrap_or_default())
        } else {
            before.clone().unwrap_or_default()
        };
        Ok(CodexHooksPreview {
            path,
            before,
            after,
            install_id,
        })
    }

    /// Apply the approved preview. If the config changed, get and show a fresh preview.
    pub fn apply_codex_hooks(&self, preview: &CodexHooksPreview) -> io::Result<()> {
        let listener = &self.codex_hook_listener;
        let _lock = listener.lock()?;
        let receipt = listener.receipt()?;
        if preview.install_id.len() != 32
            || !preview
                .install_id
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit())
            || receipt.as_ref().is_some_and(|receipt| {
                receipt.install_id != preview.install_id
                    || receipt.path.as_ref() != Some(&preview.path)
            })
            || &self.codex_hooks_preview(preview.install_id.clone())? != preview
        {
            return Err(io::Error::other(
                "Codex config changed; preview the install again (remove any install in a previous Codex home first)",
            ));
        }

        let target = settings_target(&preview.path)?;
        listener.publish()?;
        if receipt.is_none() {
            listener.save_receipt(&Receipt {
                install_id: preview.install_id.clone(),
                path: Some(preview.path.clone()),
                before: preview.before.clone(),
                after: preview.after.clone(),
            })?;
        }
        if preview.before.as_deref() != Some(preview.after.as_str()) {
            write_atomic(&target, preview.after.as_bytes())?;
        }
        Ok(())
    }

    /// Remove the installed Codex handlers, preserving the user's other settings.
    pub fn remove_codex_hooks(&self) -> io::Result<()> {
        let listener = &self.codex_hook_listener;
        if !listener.receipt_path().try_exists()? {
            return Ok(());
        }
        let _lock = listener.lock()?;
        let Some(receipt) = listener.receipt()? else {
            return Ok(());
        };
        let path = receipt
            .path
            .ok_or_else(|| invalid("missing Codex config path in receipt"))?;
        let target = settings_target(&path)?;
        if let Some(current) = read_optional(&target)? {
            if current == receipt.after {
                match receipt.before {
                    Some(before) => write_atomic(&target, before.as_bytes())?,
                    None => fs::remove_file(&target)?,
                }
            } else {
                let command = self.codex_hook_command(&receipt.install_id)?;
                let mut config = parse(Some(&current))?;
                let original = parse(receipt.before.as_deref())?;
                if remove_hooks(&mut config, &original, &command) {
                    write_atomic(&target, render(&config, &current).as_bytes())?;
                }
            }
        }
        listener.clear_installation()
    }
}

fn parse(text: Option<&str>) -> io::Result<DocumentMut> {
    text.unwrap_or_default()
        .parse()
        .map_err(|error| invalid(format!("invalid Codex config: {error}")))
}

fn render(config: &DocumentMut, previous: &str) -> String {
    let text = config.to_string();
    if previous.contains("\r\n")
        && previous
            .split_inclusive('\n')
            .all(|line| !line.ends_with('\n') || line.ends_with("\r\n"))
    {
        text.replace("\r\n", "\n").replace('\n', "\r\n")
    } else {
        text
    }
}

fn invalid(message: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message.into())
}

fn add_hooks(config: &mut DocumentMut, command: &str) -> io::Result<bool> {
    let hooks = config.entry("hooks").or_insert_with(|| {
        let mut table = Table::new();
        table.set_implicit(true);
        Item::Table(table)
    });
    let inline = hooks.as_inline_table().is_some();
    let hooks = hooks
        .as_table_like_mut()
        .ok_or_else(|| invalid("Codex hooks must be a TOML table"))?;
    let mut changed = false;
    for &(event, matcher) in EVENTS {
        let groups = hooks.entry(event).or_insert_with(|| {
            if inline {
                value(Array::new())
            } else {
                Item::ArrayOfTables(ArrayOfTables::new())
            }
        });
        if let Some(tables) = groups.as_array_of_tables_mut() {
            if !tables
                .iter()
                .any(|group| contains_handler(group, matcher, command))
            {
                tables.push(hook_group(matcher, command));
                changed = true;
            }
        } else if let Some(array) = groups.as_array_mut() {
            if !array
                .iter()
                .filter_map(Value::as_inline_table)
                .any(|group| contains_handler(group, matcher, command))
            {
                array.push(hook_group(matcher, command).into_inline_table());
                changed = true;
            }
        } else {
            return Err(invalid(format!("Codex {event} hooks must be an array")));
        }
    }
    Ok(changed)
}

fn hook_group(matcher: Option<&str>, command: &str) -> Table {
    let mut handler = Table::new();
    handler["type"] = value("command");
    handler["command"] = value(command);
    let mut handlers = ArrayOfTables::new();
    handlers.push(handler);

    let mut group = Table::new();
    if let Some(matcher) = matcher {
        group["matcher"] = value(matcher);
    }
    group["hooks"] = Item::ArrayOfTables(handlers);
    group
}

fn matches_handler(handler: &dyn TableLike, command: &str) -> bool {
    handler.get("type").and_then(Item::as_str) == Some("command")
        && handler.get("command").and_then(Item::as_str) == Some(command)
}

fn contains_handler(group: &dyn TableLike, matcher: Option<&str>, command: &str) -> bool {
    if group.get("matcher").and_then(Item::as_str) != matcher {
        return false;
    }
    match group.get("hooks") {
        Some(Item::ArrayOfTables(handlers)) => handlers.iter().any(|h| matches_handler(h, command)),
        Some(Item::Value(Value::Array(handlers))) => handlers
            .iter()
            .filter_map(Value::as_inline_table)
            .any(|h| matches_handler(h, command)),
        _ => false,
    }
}

fn remove_hooks(config: &mut DocumentMut, original: &DocumentMut, command: &str) -> bool {
    let original_hooks = original.get("hooks").and_then(Item::as_table_like);
    let Some(hooks) = config.get_mut("hooks").and_then(Item::as_table_like_mut) else {
        return false;
    };
    let mut changed = false;
    for &(event, matcher) in EVENTS {
        let Some(groups) = hooks.get_mut(event) else {
            continue;
        };
        let mut removed = false;
        let empty = match groups {
            Item::ArrayOfTables(tables) => {
                for index in (0..tables.len()).rev() {
                    if remove_handler(
                        tables.get_mut(index).unwrap(),
                        matcher,
                        command,
                        &mut removed,
                    ) {
                        tables.remove(index);
                    }
                }
                tables.is_empty()
            }
            Item::Value(Value::Array(array)) => {
                for index in (0..array.len()).rev() {
                    if let Some(group) = array.get_mut(index).and_then(Value::as_inline_table_mut) {
                        if remove_handler(group, matcher, command, &mut removed) {
                            array.remove(index);
                        }
                    }
                }
                array.is_empty()
            }
            _ => false,
        };
        if removed
            && empty
            && original_hooks
                .and_then(|original| original.get(event))
                .is_none()
        {
            hooks.remove(event);
        }
        changed |= removed;
    }
    // An explicit table header may have been added by the user after installation.
    if changed
        && original_hooks.is_none()
        && config
            .get("hooks")
            .and_then(Item::as_table)
            .is_some_and(|table| table.is_empty() && table.is_implicit())
    {
        config.remove("hooks");
    }
    changed
}

/// Returns whether the group itself should be removed.
fn remove_handler(
    group: &mut dyn TableLike,
    matcher: Option<&str>,
    command: &str,
    removed: &mut bool,
) -> bool {
    let Some(handlers) = group.get_mut("hooks") else {
        return false;
    };
    let (count, remaining) = match handlers {
        Item::ArrayOfTables(tables) => {
            let count = tables.len();
            tables.retain(|h| !matches_handler(h, command));
            (count, tables.len())
        }
        Item::Value(Value::Array(array)) => {
            let count = array.len();
            array.retain(|h| {
                !h.as_inline_table()
                    .is_some_and(|h| matches_handler(h, command))
            });
            (count, array.len())
        }
        _ => return false,
    };
    if remaining == count {
        return false;
    }
    *removed = true;
    if remaining != 0 {
        return false;
    }
    if group.len() == 1 + usize::from(matcher.is_some())
        && group.get("matcher").and_then(Item::as_str) == matcher
    {
        return true;
    }
    // An empty array of tables would serialize without the required `hooks` key.
    if group.get("hooks").is_some_and(Item::is_array_of_tables) {
        group.insert("hooks", value(Array::new()));
    }
    false
}
