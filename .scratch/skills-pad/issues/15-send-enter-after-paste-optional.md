# 15: Send Enter after paste (optional)

**What to build:** An optional toggle in the **Pad**, off by default, that makes every **Paste** finish with Enter so a tap runs the **Skill** instead of only inserting its **Invocation**.

**Blocked by:** #5

**Status:** done (manual checks pending)

**GitHub:** #16

- [x] With the toggle off, behavior is unchanged: only the Invocation and trailing space are pasted
- [x] With it on, Enter is sent after the paste on macOS and Windows
- [x] The toggle persists across restarts and applies to the next tap without restarting
- [x] Serialized pastes from several phones each get their own Enter, in order (tested with the recording Paster)

**Notes:** The Core has `Core::settings()` and `Core::set_send_enter(bool)`, saved to `settings.json` in the OS config folder (`settings::config_dir`). Unknown fields in that file are kept, so the shell can store its own settings, such as the hotkey, in the same file. `Paster::press_enter` is implemented by `SystemPaster` (enigo Return) and recorded by `RecordingPaster` as `Recorded::Enter`. The Enter on Windows is untested on real hardware.

The Pad has a Send Enter toggle (commit 5db9e21). Manual check: turn it on and confirm a tap runs the skill in a real terminal.
