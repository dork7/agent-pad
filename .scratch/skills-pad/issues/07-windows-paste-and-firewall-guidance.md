# 07: Windows paste and firewall guidance

**What to build:** The same tap-to-paste works on Windows using Ctrl+V. The Pad explains the first-run firewall prompt so you know to allow it.

**Blocked by:** #5

**Status:** ready-for-agent

**GitHub:** #8

- [ ] Tapping a skill pastes its Invocation into the focused Windows app
- [ ] Clipboard restore and serialized pastes behave as on macOS
- [ ] The Pad shows plain guidance about the firewall prompt before first sharing
- [ ] The hotkey and tray icon work on Windows

**Notes (as of 2026-11-25):** The paste flow already uses Ctrl+V on Windows (`SystemPaster`), and the queue and clipboard restore are the same code as on macOS. `skills-pad-core` passes `cargo check --target x86_64-pc-windows-msvc` from macOS. The app crate can't be cross-checked from macOS because `tauri-winres` needs a Windows resource compiler. Still needed, on a Windows machine: build the app, check real pastes, the hotkey and the tray, and add the firewall guidance card in the Pad. The hook forwarders (tickets 13 and 14) use a POSIX shell and curl, so they also need a native Windows command.
