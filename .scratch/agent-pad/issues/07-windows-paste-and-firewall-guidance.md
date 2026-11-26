# 07: Windows paste and firewall guidance

**What to build:** The same tap-to-paste works on Windows using Ctrl+V. The Console explains the first-run firewall prompt so you know to allow it.

**Blocked by:** #5

**Status:** ready-for-agent

**GitHub:** #8

- [ ] Tapping a skill pastes its Invocation into the focused Windows app
- [ ] Clipboard restore and serialized pastes behave as on macOS
- [x] The Console shows plain guidance about the firewall prompt before first sharing
- [ ] The hotkey and tray icon work on Windows

**Notes (as of 2026-11-26):** The paste flow already uses Ctrl+V on Windows (`SystemPaster`), and the queue and clipboard restore use the same code as on macOS. Hook installation now renders a native PowerShell forwarder on Windows for Claude Code and Codex; the POSIX forwarder is unchanged. The Console shows firewall guidance on Windows before the first successful sharing session. Unit tests render the Windows command and pass. `cargo test --workspace` reached the integration tests, which cannot open local listeners in this sandbox (`Operation not permitted`). The requested Windows cross-check could not run here: the target is missing and `rustup target add x86_64-pc-windows-msvc` could not download it because DNS access is blocked. A prior macOS cross-check of `agent-pad-core` passed; the app crate still needs a Windows resource compiler. On a real Windows machine, build the app and test real pastes, clipboard restoration and serialization, the hotkey and tray, the first-run firewall prompt and phone connection, and both installed hook forwarders (including removal and offline behavior).
