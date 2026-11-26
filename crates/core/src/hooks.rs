//! Claude Code and Codex command hooks forward stdin to the loopback listener. After consent,
//! Core publishes its PID and current port atomically in the OS config directory
//! under the configured home. The command checks the PID, bypasses proxies and curl
//! config, and exits quietly with a short timeout. Core removes its discovery file on
//! shutdown. Windows uses PowerShell so hooks do not require Git Bash or curl.
//!
//! A private receipt records the original settings text and the identity of Skills
//! Pad's handlers. Removal can restore the original bytes when untouched and remove
//! only those handlers when settings have been edited since installation.

use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Write};
use std::net::SocketAddr;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tempfile::NamedTempFile;
use tiny_http::{Request, Response};

use crate::{server, sessions::Event, Core, State};

mod codex;
pub use codex::{CodexHooksPreview, CODEX_HOOKS_RESTART_NOTICE};

// Event names, matchers and stdin fields: https://code.claude.com/docs/en/hooks
const EVENTS: &[(&str, Option<&str>)] = &[
    ("SessionStart", Some("startup|resume|clear|fork")),
    ("UserPromptSubmit", None),
    ("PreToolUse", None),
    ("PostToolUse", None),
    ("PostToolUseFailure", None),
    ("PermissionRequest", None),
    ("Notification", Some("permission_prompt")),
    ("Stop", None),
    ("SessionEnd", None),
];

#[derive(Clone, Copy)]
pub(crate) enum Harness {
    Claude,
    Codex,
}

impl Harness {
    fn id(self) -> &'static str {
        match self {
            Self::Claude => "claude",
            Self::Codex => "codex",
        }
    }

    fn name(self) -> &'static str {
        match self {
            Self::Claude => "Claude Code",
            Self::Codex => "Codex",
        }
    }
}

#[derive(Deserialize)]
struct HookEvent {
    session_id: String,
    cwd: PathBuf,
    hook_event_name: String,
    source: Option<String>,
    notification_type: Option<String>,
    is_interrupt: Option<bool>,
}

pub(crate) fn receive(
    state: &State,
    request: &mut Request,
    harness: Harness,
) -> Response<io::Cursor<Vec<u8>>> {
    if !is_loopback(request.remote_addr().copied()) {
        return server::json_status(403, &json!({"error": "loopback required"}));
    }
    // Browsers cannot send this content type cross-origin without a preflight.
    let json_content = request.headers().iter().any(|header| {
        header.field.equiv("Content-Type")
            && header
                .value
                .as_str()
                .split(';')
                .next()
                .is_some_and(|value| value.trim().eq_ignore_ascii_case("application/json"))
    });
    if !json_content {
        return server::json_status(415, &json!({"error": "application/json required"}));
    }

    const MAX_BODY: u64 = 16 * 1024 * 1024;
    let mut body = Vec::new();
    if request
        .as_reader()
        .take(MAX_BODY + 1)
        .read_to_end(&mut body)
        .is_err()
    {
        return server::json_status(400, &json!({"error": "could not read hook event"}));
    }
    if body.len() as u64 > MAX_BODY {
        return server::json_status(413, &json!({"error": "hook event too large"}));
    }
    let Ok(payload) = serde_json::from_slice::<HookEvent>(&body) else {
        let error = format!("invalid {} hook event", harness.name());
        return server::json_status(400, &json!({"error": error}));
    };
    let event = match harness {
        Harness::Claude => claude_event(&payload),
        Harness::Codex => codex::event(&payload),
    };
    if let Some(event) = event {
        let process_id = request.headers().iter()
            .find(|header| header.field.equiv("X-Agent-Pad-Process-Id"))
            .and_then(|header| header.value.as_str().parse::<u32>().ok())
            .filter(|id| *id > 0);
        state.sessions.lock().unwrap().record(
            harness.id(),
            payload.session_id,
            payload.cwd,
            process_id,
            event,
            state.clock.now(),
        );
    }
    Response::from_data(Vec::new()).with_status_code(204)
}

fn claude_event(payload: &HookEvent) -> Option<Event> {
    match payload.hook_event_name.as_str() {
        "SessionStart" if payload.source.as_deref() != Some("compact") => Some(Event::Start),
        "UserPromptSubmit" | "PostToolUse" => Some(Event::Working),
        "PreToolUse" => Some(Event::ToolStarted),
        "PermissionRequest" => Some(Event::Permission),
        "Notification" if payload.notification_type.as_deref() == Some("permission_prompt") => {
            Some(Event::Permission)
        }
        "PostToolUseFailure" if payload.is_interrupt == Some(true) => Some(Event::Stop),
        "PostToolUseFailure" => Some(Event::Working),
        "Stop" => Some(Event::Stop),
        "SessionEnd" => Some(Event::End),
        _ => None,
    }
}

fn shell_quote(text: &str) -> String {
    format!("'{}'", text.replace('\'', "'\\''"))
}

fn windows_hook_script(path: &str, harness: &str) -> String {
    let path = path.replace('\'', "''");
    format!(
        "$ErrorActionPreference = 'Stop'; try {{ \
         $line = [IO.File]::ReadAllText('{path}').Trim(); \
         if ($line -notmatch '^([0-9]+) ([0-9]+)$') {{ exit 0 }}; \
         $targetPid = [int]$Matches[1]; $port = [int]$Matches[2]; \
         if ($port -lt 1 -or $port -gt 65535) {{ exit 0 }}; \
         Get-Process -Id $targetPid -ErrorAction Stop | Out-Null; \
         $inputBytes = New-Object IO.MemoryStream; \
         [Console]::OpenStandardInput().CopyTo($inputBytes); \
         $handler = New-Object Net.Http.HttpClientHandler; $handler.UseProxy = $false; \
         $client = [Net.Http.HttpClient]::new($handler); \
         $client.Timeout = [TimeSpan]::FromMilliseconds(500); \
         $body = [Net.Http.ByteArrayContent]::new($inputBytes.ToArray()); \
         $body.Headers.ContentType = [Net.Http.Headers.MediaTypeHeaderValue]::Parse('application/json'); \
         $parentPid = (Get-CimInstance Win32_Process -Filter ('ProcessId=' + $PID)).ParentProcessId; \
         if ($parentPid -gt 0) {{ $body.Headers.Add('X-Agent-Pad-Process-Id', [string]$parentPid) }}; \
         $response = $client.PostAsync('http://127.0.0.1:' + $port + '/hooks/{harness}', $body).GetAwaiter().GetResult(); \
         $response.Dispose(); $body.Dispose(); $client.Dispose(); $handler.Dispose(); \
         }} catch {{ }}"
    )
}

fn windows_hook_command(path: &str, harness: &str, install_id: &str) -> String {
    // -EncodedCommand avoids cmd.exe expanding %, &, and other characters in the home path.
    let script = windows_hook_script(path, harness);
    let bytes: Vec<u8> = script.encode_utf16().flat_map(u16::to_le_bytes).collect();
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut encoded = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let n = ((chunk[0] as u32) << 16)
            | ((chunk.get(1).copied().unwrap_or(0) as u32) << 8)
            | chunk.get(2).copied().unwrap_or(0) as u32;
        encoded.push(ALPHABET[((n >> 18) & 63) as usize] as char);
        encoded.push(ALPHABET[((n >> 12) & 63) as usize] as char);
        encoded.push(if chunk.len() > 1 {
            ALPHABET[((n >> 6) & 63) as usize] as char
        } else {
            '='
        });
        encoded.push(if chunk.len() > 2 {
            ALPHABET[(n & 63) as usize] as char
        } else {
            '='
        });
    }
    format!("powershell.exe -NoProfile -NonInteractive -EncodedCommand {encoded} >nul 2>&1 & rem skills-pad-{install_id}")
}

fn is_loopback(peer: Option<SocketAddr>) -> bool {
    peer.is_some_and(|addr| addr.ip().is_loopback())
}

/// Exact settings text to display before asking for consent. A missing file has
/// `before: None`. Pass the complete preview back to apply it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClaudeHooksPreview {
    pub path: PathBuf,
    pub before: Option<String>,
    pub after: String,
    install_id: String,
}

#[derive(Serialize, Deserialize)]
struct Receipt {
    install_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    path: Option<PathBuf>,
    before: Option<String>,
    after: String,
}

pub(crate) struct Listener {
    directory: PathBuf,
    harness: Harness,
    port: u16,
}

impl Listener {
    pub(crate) fn new(home: &Path, port: u16, harness: Harness) -> io::Result<Self> {
        let listener = Self {
            directory: std::path::absolute(crate::settings::config_dir(home))?,
            harness,
            port,
        };
        if listener.receipt_path().try_exists()? {
            let _lock = listener.lock()?;
            if listener.receipt_path().try_exists()? {
                listener.publish()?;
            }
        }
        Ok(listener)
    }

    fn discovery_path(&self) -> PathBuf {
        self.directory
            .join(format!("{}-listener", self.harness.id()))
    }

    fn receipt_path(&self) -> PathBuf {
        self.directory
            .join(format!("{}-hooks.json", self.harness.id()))
    }

    fn lock(&self) -> io::Result<File> {
        fs::create_dir_all(&self.directory)?;
        let lock = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(false)
            .open(
                self.directory
                    .join(format!("{}-hooks.lock", self.harness.id())),
            )?;
        lock.lock()?;
        Ok(lock)
    }

    fn discovery(&self) -> String {
        format!("{} {}\n", std::process::id(), self.port)
    }

    fn publish(&self) -> io::Result<()> {
        write_atomic(&self.discovery_path(), self.discovery().as_bytes())
    }

    fn command(&self, install_id: &str) -> io::Result<String> {
        let discovery = self.discovery_path();
        let path = discovery.to_str().ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "hook discovery path must be UTF-8",
            )
        })?;
        let harness = self.harness.id();
        if cfg!(windows) {
            return Ok(windows_hook_command(path, harness, install_id));
        }
        let path = shell_quote(path);
        // The trailing "skills-pad-<id>" marker keeps its pre-rename spelling on purpose: removal
        // finds hooks by it, including ones installed before the rename to Agent Pad.
        Ok(format!(
            "if [ -f {path} ] && IFS=' ' read -r pid port < {path}; then \
             case \"$pid:$port\" in *[!0-9:]*|:*|*:) exit 0;; esac; \
             if kill -0 \"$pid\"; then \
             curl -q --silent --noproxy '*' --max-time 0.5 \
             -H 'Content-Type: application/json' -H \"X-Agent-Pad-Process-Id: $PPID\" --data-binary @- \
             \"http://127.0.0.1:$port/hooks/{harness}\"; fi; fi >/dev/null 2>&1; \
             exit 0 # skills-pad-{install_id}"
        ))
    }

    fn install_id(&self) -> io::Result<String> {
        if let Some(receipt) = self.receipt()? {
            return Ok(receipt.install_id);
        }
        let mut bytes = [0; 16];
        getrandom::fill(&mut bytes).map_err(io::Error::other)?;
        Ok(format!("{:032x}", u128::from_be_bytes(bytes)))
    }

    fn save_receipt(&self, receipt: &Receipt) -> io::Result<()> {
        let bytes = serde_json::to_vec(receipt).map_err(io::Error::other)?;
        write_atomic(&self.receipt_path(), &bytes)
    }

    fn clear_installation(&self) -> io::Result<()> {
        // Stop forwarding from processes still using a previously loaded config.
        if self.discovery_path().try_exists()? {
            fs::remove_file(self.discovery_path())?;
        }
        fs::remove_file(self.receipt_path())
    }

    fn receipt(&self) -> io::Result<Option<Receipt>> {
        read_optional(&self.receipt_path())?
            .map(|text| serde_json::from_str(&text).map_err(io::Error::other))
            .transpose()
    }
}

impl Drop for Listener {
    fn drop(&mut self) {
        let path = self.discovery_path();
        if !path.exists() {
            return;
        }
        let Ok(_lock) = self.lock() else {
            return;
        };
        if fs::read_to_string(&path).is_ok_and(|current| current == self.discovery()) {
            let _ = fs::remove_file(path);
        }
    }
}

impl Core {
    /// Compute the exact install change without writing files.
    pub fn preview_claude_hooks(&self) -> io::Result<ClaudeHooksPreview> {
        self.claude_hooks_preview(self.hook_listener.install_id()?)
    }

    fn claude_hooks_preview(&self, install_id: String) -> io::Result<ClaudeHooksPreview> {
        let path = self.state.home.join(".claude/settings.json");
        let before = read_optional(&settings_target(&path)?)?;
        let command = self.hook_listener.command(&install_id)?;
        let (settings, changed) = add_hooks(parse_settings(before.as_deref())?, &command)?;
        let after = if changed {
            render(&settings)?
        } else {
            before.clone().unwrap_or_default()
        };
        Ok(ClaudeHooksPreview {
            path,
            before,
            after,
            install_id,
        })
    }

    /// Apply the approved preview. If settings changed, get and show a fresh preview.
    pub fn apply_claude_hooks(&self, preview: &ClaudeHooksPreview) -> io::Result<()> {
        let listener = &self.hook_listener;
        let _lock = listener.lock()?;
        let receipt = listener.receipt()?;
        if preview.install_id.len() != 32
            || !preview
                .install_id
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit())
            || receipt
                .as_ref()
                .is_some_and(|receipt| receipt.install_id != preview.install_id)
            || &self.claude_hooks_preview(preview.install_id.clone())? != preview
        {
            return Err(io::Error::other(
                "Claude Code settings changed; preview the install again",
            ));
        }
        listener.publish()?;
        if receipt.is_none() {
            let receipt = Receipt {
                install_id: preview.install_id.clone(),
                path: None,
                before: preview.before.clone(),
                after: preview.after.clone(),
            };
            listener.save_receipt(&receipt)?;
        }
        if preview.before.as_deref() != Some(preview.after.as_str()) {
            write_atomic(&settings_target(&preview.path)?, preview.after.as_bytes())?;
        }
        Ok(())
    }

    /// Remove the installed handlers while preserving any other settings.
    pub fn remove_claude_hooks(&self) -> io::Result<()> {
        let listener = &self.hook_listener;
        if !listener.receipt_path().try_exists()? {
            return Ok(());
        }
        let _lock = listener.lock()?;
        let Some(receipt) = listener.receipt()? else {
            return Ok(());
        };
        let path = settings_target(&self.state.home.join(".claude/settings.json"))?;
        if let Some(current) = read_optional(&path)? {
            if current == receipt.after {
                match receipt.before {
                    Some(before) => write_atomic(&path, before.as_bytes())?,
                    None => fs::remove_file(&path)?,
                }
            } else {
                let mut settings = parse_settings(Some(&current))?;
                let original = parse_settings(receipt.before.as_deref())?;
                let command = listener.command(&receipt.install_id)?;
                if remove_hooks(&mut settings, &original, &command) {
                    write_atomic(&path, render(&settings)?.as_bytes())?;
                }
            }
        }
        listener.clear_installation()
    }
}

fn group(matcher: Option<&str>, hooks: Vec<Value>) -> Value {
    match matcher {
        Some(matcher) => json!({"matcher": matcher, "hooks": hooks}),
        None => json!({"hooks": hooks}),
    }
}

fn add_hooks(mut settings: Value, command: &str) -> io::Result<(Value, bool)> {
    let settings_map = settings.as_object_mut().expect("validated settings");
    let hooks = settings_map
        .entry("hooks")
        .or_insert_with(|| json!({}))
        .as_object_mut()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "hooks must be a JSON object"))?;
    let handler = json!({"type": "command", "command": command});
    let mut changed = false;
    for &(event, matcher) in EVENTS {
        let groups = hooks
            .entry(event)
            .or_insert_with(|| json!([]))
            .as_array_mut()
            .ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("{event} hooks must be an array"),
                )
            })?;
        let installed = groups.iter().any(|group| {
            group.get("matcher").and_then(Value::as_str) == matcher
                && group
                    .get("hooks")
                    .and_then(Value::as_array)
                    .is_some_and(|handlers| handlers.iter().any(|hook| is_handler(hook, command)))
        });
        if !installed {
            groups.push(group(matcher, vec![handler.clone()]));
            changed = true;
        }
    }
    Ok((settings, changed))
}

fn remove_hooks(settings: &mut Value, original: &Value, command: &str) -> bool {
    let Some(hooks) = settings.get_mut("hooks").and_then(Value::as_object_mut) else {
        return false;
    };
    let mut changed = false;
    for &(event, matcher) in EVENTS {
        let Some(groups) = hooks.get_mut(event).and_then(Value::as_array_mut) else {
            continue;
        };
        let mut removed = false;
        groups.retain_mut(|entry| {
            let Some(handlers) = entry.get_mut("hooks").and_then(Value::as_array_mut) else {
                return true;
            };
            let count = handlers.len();
            handlers.retain(|hook| !is_handler(hook, command));
            if handlers.len() == count {
                return true;
            }
            removed = true;
            *entry != group(matcher, Vec::new())
        });
        // Preserve any empty containers the user already had.
        changed |= removed;
        if removed && groups.is_empty() && original["hooks"].get(event).is_none() {
            hooks.remove(event);
        }
    }
    if changed && hooks.is_empty() && original.get("hooks").is_none() {
        settings.as_object_mut().unwrap().remove("hooks");
    }
    changed
}

fn is_handler(hook: &Value, command: &str) -> bool {
    hook["type"] == "command" && hook["command"] == command
}

fn parse_settings(text: Option<&str>) -> io::Result<Value> {
    let settings: Value = serde_json::from_str(text.unwrap_or("{}"))?;
    if !settings.is_object() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Claude Code settings must be a JSON object",
        ));
    }
    Ok(settings)
}

fn render(value: &Value) -> io::Result<String> {
    Ok(serde_json::to_string_pretty(value)? + "\n")
}

fn settings_target(path: &Path) -> io::Result<PathBuf> {
    match path.canonicalize() {
        Ok(path) => Ok(path), // Write through symlinks to dotfiles.
        Err(error) if error.kind() == io::ErrorKind::NotFound => match fs::symlink_metadata(path) {
            Err(missing) if missing.kind() == io::ErrorKind::NotFound => Ok(path.to_path_buf()),
            _ => Err(error),
        },
        Err(error) => Err(error),
    }
}

fn read_optional(path: &Path) -> io::Result<Option<String>> {
    match fs::read_to_string(path) {
        Ok(text) => Ok(Some(text)),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error),
    }
}

fn write_atomic(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let directory = path.parent().expect("path has parent");
    fs::create_dir_all(directory)?;
    let mut temp = NamedTempFile::new_in(directory)?;
    if let Ok(metadata) = fs::metadata(path) {
        temp.as_file().set_permissions(metadata.permissions())?;
    }
    temp.write_all(bytes)?;
    temp.as_file().sync_all()?;
    temp.persist(path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn peer_must_actually_be_loopback() {
        assert!(is_loopback(Some("127.0.0.1:1234".parse().unwrap())));
        assert!(is_loopback(Some("[::1]:1234".parse().unwrap())));
        assert!(!is_loopback(Some("192.0.2.1:1234".parse().unwrap())));
        assert!(!is_loopback(None));
    }

    #[test]
    fn windows_command_renders_native_forwarder() {
        let command = windows_hook_command(
            r"C:\Users\O'Brien & Co\agent-pad\claude-listener",
            "claude",
            "abc123",
        );
        assert!(command.starts_with("powershell.exe -NoProfile -NonInteractive -EncodedCommand "));
        assert!(command.ends_with(" >nul 2>&1 & rem skills-pad-abc123"));
        assert!(!command.contains("curl"));
        assert!(!command.contains("/bin/sh"));
        let script =
            windows_hook_script(r"C:\Users\O'Brien & Co\agent-pad\claude-listener", "claude");
        assert!(script.contains(r"ReadAllText('C:\Users\O''Brien & Co\agent-pad\claude-listener')"));
        assert!(script.contains("/hooks/claude"));
        assert!(script.contains("UseProxy = $false"));
        assert!(script.contains("FromMilliseconds(500)"));
        assert!(script.contains("OpenStandardInput().CopyTo($inputBytes)"));
        assert!(windows_hook_script(r"C:\test\codex-listener", "codex").contains("/hooks/codex"));
    }
}
