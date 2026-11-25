# 14: Codex session status and view filtering of sessions

**What to build:** The same session status for Codex, using its hooks. A phone's **View** now filters **Sessions** by harness as well as skills.

**Blocked by:** #12, #14

**Status:** done (manual checks pending)

**GitHub:** #15

- [x] Codex events (including permission requests and interrupts) produce the same Status states
- [x] A phone assigned only Codex sees only Codex sessions
- [x] Changing a phone's View updates its visible sessions live
- [x] Cursor, Gemini CLI, OpenCode and Copilot sessions are not shown
- [x] The Pad displays the Codex config path and diff before installation, requires a fresh preview after a stale one, and offers removal and the hook trust notice.

## Notes

- The Pad should display `path`, `before`, and `after` from `Core::preview_codex_hooks()`, then pass that preview to `Core::apply_codex_hooks(&preview)` after consent. A stale preview requires a fresh review. `Core::remove_codex_hooks()` restores untouched config byte for byte and removes Skills Pad's handlers from edited config.
- Core uses inline command hooks in `~/.codex/config.toml` (or `$CODEX_HOME/config.toml` when configured). After installation, display `CODEX_HOOKS_RESTART_NOTICE`. Codex requires users to review and trust new hooks in `/hooks` in the CLI or in the desktop hook settings. Skills Pad leaves Codex's feature flags and hook trust settings under the user's control. [Codex hooks](https://developers.openai.com/codex/hooks) · [Configuration reference](https://developers.openai.com/codex/config-reference)
- The hooks send documented JSON events to the loopback listener. Start or resume registers a session. Prompt and tool activity mark it working, a permission request marks it as needing input, stop and interrupt mark it idle, and session end marks it ended. The shared working-to-stuck threshold also applies. The forwarder uses a POSIX shell and curl. Native Windows forwarding remains pending.
- The phone's authenticated `GET /sessions` reads its current View on each request. A null View includes all tracked sessions. Harness chips select sessions by harness, and directory chips select skills only.

## Manual checks pending

- [ ] Preview, install, and remove hooks against a test Codex configuration. Confirm the trust notice, review and trust the hooks through `/hooks`, then start or resume a session and verify events appear in the Pad.
- [ ] Pair phones with Claude-only and Codex-only Views on the integrated build and switch their Views while the sessions run.
