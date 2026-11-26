# 05: Console shell: hotkey and status-bar icon

**What to build:** A global hotkey (default Cmd+Option+S on macOS, Ctrl+Alt+S on Windows, rebindable) toggles the **Console**, and a status-bar icon with a menu provides the same actions. The Console does not stay on top of other windows.

**Blocked by:** #4

**Status:** done (manual checks pending)

**GitHub:** #6

- [ ] The hotkey shows and hides the Console
- [x] The status-bar icon menu offers open Console, Open on phone, Stop sharing and Quit
- [ ] The hotkey can be rebound in the Console and the change takes effect without restart
- [x] The Console is a normal window, not always-on-top

## Manual checks

- With another app focused, press Cmd+Option+S on macOS or Ctrl+Alt+S on Windows to show and hide the Console. Close the Console, check that it stays available from the status bar, and confirm that another window can cover it. On macOS, check that Agent Pad has no Dock icon.
- Save a new hotkey, confirm that it takes effect immediately and that the old hotkey is released, then relaunch to check persistence. Try malformed input and an occupied global shortcut, and confirm that the current hotkey still works.
- Click Open Console, Open on phone, Stop sharing, and Quit in the status-bar menu. Check that Open on phone reveals the QR code and matching URL in the Console and that Stop sharing clears them. Repeat the sharing actions from the Console.

The macOS conflict probe was verified against an enabled system shortcut and an exclusively registered global shortcut. macOS allows applications to share nonexclusive global registrations.
