# 27: Verify the Agent Pad rename builds, tests and migrates

**What to build:** The product was renamed from Skills Pad to Agent Pad without compiling, because Rust was not installed on the machine that made the change. Install the toolchain, build and test the whole workspace, and confirm that an existing install carries over.

**Blocked by:** None (can start immediately)

**Status:** done

**GitHub:** #25

- [x] `cargo build` and `cargo test` pass for the whole workspace, with the regenerated lockfile committed
- [x] A settings folder left by the old name is moved to the new one on first launch, with a test using a temp home directory
- [x] Cookie names and the hook marker keep their old spelling on purpose, so already-paired phones and already-installed hooks keep working; hook removal still works for hooks installed before the rename
- [x] Document that the new bundle identifier means macOS asks for Accessibility access again and a start-at-login item must be re-enabled
- [x] The tray tooltip, window title, phone page title and docs all say Agent Pad
