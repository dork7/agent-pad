# 06: First-run: macOS Accessibility guidance and start-at-login

**What to build:** On macOS the Pad detects missing Accessibility permission, offers a "Grant access" button that opens System Settings, and blocks sharing until it is granted. An optional start-at-login toggle is added (off by default).

**Blocked by:** #5, #6

**Status:** done (manual checks pending)

**GitHub:** #7

- [ ] Without permission, "Open on phone" is disabled with a clear explanation
- [ ] After granting, sharing becomes available without reinstalling
- [ ] Start-at-login can be turned on and off from the Pad

## Verified

- [x] `cargo clippy --workspace --all-targets -- -D warnings` and `cargo test --workspace` passed on macOS (55 tests).
- [x] The Pad showed the guidance and disabled sharing with mocked denied trust, requested System Settings on Grant access, and enabled sharing when a poll reported trusted access. A focus change picked up revoked trust.
- [x] With mocked autostart commands, the login toggle started off, read back the state after enabling and disabling, and restored the plugin state after a failed write.
- [x] `./target/debug/skills-pad-app` stayed running for eight seconds and was then stopped.

## Manual checks

- [ ] Start with Accessibility access off. Check that Open on phone is disabled in the Pad and the tray menu, and that the Pad explains why pasting needs access. Click Grant access and confirm System Settings opens to Privacy & Security → Accessibility. Turn on Skills Pad there and confirm both sharing actions become available without restarting or reinstalling. Start sharing and verify a phone tap pastes an invocation.
- [ ] With a fresh install, confirm Start at login is off. Turn it on in the Pad, quit and relaunch, then confirm it is still on. Turn it off and repeat the relaunch.
- [ ] With Start at login enabled, find Skills Pad's entry in System Settings → General → Login Items, where macOS may list the LaunchAgent under background items. Confirm the entry is removed after turning the toggle off.
