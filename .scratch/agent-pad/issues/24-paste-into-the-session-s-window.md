# 24: Paste into the session's window

**What to build:** When **Run** is chosen for a skill on a **Session screen**, the session's terminal or editor window is brought to the front before the paste. If the window can't be found, nothing is pasted and the phone says so; it never falls back to the focused app (ADR 0002). Matching is at window level, not tab level, on macOS and Windows.

**Blocked by:** #14, #5, #8

**Status:** done (manual checks pending)

**GitHub:** #22

- [x] The hook events' project folder and process are used to find the owning window; nothing arbitrary is accepted from the phone
- [ ] On macOS and Windows the matched window is raised and focus returned before the clipboard paste and Enter (if enabled)
- [x] When no window matches, nothing is pasted or sent and the phone says the window was not found
- [x] The "focus the session's terminal first" dialog is no longer needed when matching succeeds
- [x] Window raising is behind a replaceable interface so tests use a recording fake
- [ ] Manual checks on macOS and Windows with a terminal and an editor; tmux and VS Code panes are documented as window-level only

Window matching is deliberately conservative: the hook supplies the session process ID and project folder, and Run walks that process's host ancestry. It raises a host only when that host has exactly one visible window. tmux sessions and VS Code panes share a window and cannot be targeted separately; if multiple windows are possible, Run refuses. Hooks installed before this change must be reinstalled to add the process ID header.
