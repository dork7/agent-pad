# 13: Claude Code session status with hook consent

**What to build:** The Pad offers to install Claude Code hooks after showing the exact config change, with one-click removal. A local-only listener receives events and derives each **Session**'s **Status** (working, needs input, idle, ended, stuck). Session cards with project folder, status and elapsed time appear on the phone page and in the Pad.

**Blocked by:** #5, #6

**Status:** done (manual checks pending)

**GitHub:** #14

- [x] No config is changed until the user consents, and the exact change is shown first
  - [x] Core returns the exact settings preview without writing and applies only a current preview.
- [x] One click removes the installed hooks cleanly
  - [x] Core removes its handlers and preserves other settings and hooks.
- [x] Status transitions follow the hook events (prompt or tool use is working, permission request is needs input, stop is idle, session end is ended)
- [x] Working with no events for the threshold (default 5 minutes, configurable) becomes stuck, tested with the fake clock
- [x] Sessions started before installation are not shown until they restart, and the Pad says so
  - [x] Core waits for a start or resume event and exposes the restart notice.
- [x] The hook listener accepts connections from the local machine only
- [x] The Pad polls sessions every two seconds while Agents is visible and shows the project, harness, status, elapsed time, and tool count.
- [x] The Pad saves and reapplies the stuck threshold across restarts.

## Notes

- The Pad calls `Core::preview_claude_hooks()` and displays `path`, `before`, and `after`. Pass the complete preview to `Core::apply_claude_hooks(&preview)` after consent. If settings changed in the meantime, show a fresh preview. Use `Core::remove_claude_hooks()` for one-click removal.
- Display `CLAUDE_HOOKS_RESTART_NOTICE` after installation. Core waits for a Claude Code `SessionStart` from startup, resume, clear, or fork. A compaction does not register a session.
- The Pad can call `Core::list_sessions()`. Both the local API and the phone sharing server expose `GET /sessions`. The phone uses its pairing cookie or the current sharing token. Each session has `session_id`, `harness` (`claude`), `project_folder`, `status` (`working`, `needs_input`, `idle`, `ended`, or `stuck`), `elapsed_seconds`, and `tool_count`.
- The default stuck threshold is five minutes. Apply the saved preference via `Core::set_stuck_threshold(Duration)` when starting Core and when the preference changes. Core keeps sessions in memory, and ended sessions retain their final elapsed time for one minute.
- Installed Claude Code commands send JSON to the local `POST /hooks/claude` route. Core publishes the current listener port under its configured home when hooks are installed. Forwarding uses a POSIX shell and curl. Native Windows forwarding is a TODO.

This branch persists the threshold in the Pad’s app config because its Core setter is in-memory. When integrating with the Core persistence on `build/specs`, reconcile the two settings stores.

## Manual checks pending

- [ ] Preview, install, and remove hooks against a test Claude Code configuration. Edit the file between Preview and Install and confirm the Pad requires a fresh preview. Verify unrelated settings survive removal.
- [ ] Restart or resume a real Claude Code session. Check working, needs input, idle, ended, and stuck, along with elapsed time and tool count. Set a short threshold, restart the Pad, and confirm the threshold persists. Check the session on a paired phone with the phone UI from build/specs.
