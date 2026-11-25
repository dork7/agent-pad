# 13: Claude Code session status with hook consent

**What to build:** The Pad offers to install Claude Code hooks after showing the exact config change, with one-click removal. A local-only listener receives events and derives each **Session**'s **Status** (working, needs input, idle, ended, stuck). Session cards with project folder, status and elapsed time appear on the phone page and in the Pad.

**Blocked by:** #5, #6

**Status:** ready-for-agent

**GitHub:** #14

- [ ] No config is changed until the user consents, and the exact change is shown first
- [ ] One click removes the installed hooks cleanly
- [ ] Status transitions follow the hook events (prompt or tool use is working, permission request is needs input, stop is idle, session end is ended)
- [ ] Working with no events for the threshold (default 5 minutes, configurable) becomes stuck, tested with the fake clock
- [ ] Sessions started before installation are not shown until they restart, and the Pad says so
- [ ] The hook listener accepts connections from the local machine only
