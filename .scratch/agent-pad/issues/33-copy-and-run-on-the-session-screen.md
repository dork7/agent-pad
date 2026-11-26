# 33: Copy and Run on the Session screen

**What to build:** Each skill on a **Session screen** expands into **Copy**, **Run** and its presets. Copy puts the **Invocation** on the computer's clipboard and does nothing else. Run raises the session's window, pastes the Invocation, presses Enter and restores the previous clipboard; if the window can't be matched it does nothing and says so. The old one-tap paste and the global Send Enter setting are removed. All skills (no session) offers Copy only. See ADR 0002.

**Blocked by:** #21, #22

**Status:** ready-for-agent

**GitHub:** #31

- [ ] Tapping a skill row expands it to show Copy, Run and any presets; a single tap never runs anything
- [ ] Copy sets the computer's clipboard to the Invocation (plus preset) with its trailing space and does not paste or press Enter; the phone shows "Copied"
- [ ] Run raises the session's window, pastes, presses Enter and restores the previous clipboard; the phone shows "Running <skill> in <project>"
- [ ] When the window can't be matched, Run does nothing and the phone says the window was not found; it never falls back to the focused app
- [ ] The one-tap paste and the Send Enter setting are removed, and an existing saved Send Enter value is ignored without error
- [ ] The phone sends a skill ID, an optional preset ID and an action only; the Core refuses Run for a session outside the phone's View
- [ ] All skills shows Copy only
- [ ] Covered by Core tests with the recording Paster and window-raise fakes, plus a manual check on macOS and Windows
- [ ] Run and Implement are disabled while the session is Blocked, with "Answer the request first" shown, and the Core refuses them too
