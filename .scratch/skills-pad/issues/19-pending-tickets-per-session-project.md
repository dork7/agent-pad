# 19: Pending tickets for a session's project

**What to build:** Tapping any **Session** card, on the phone or in the Pad, offers "Pending tickets". This opens a list of the not-yet-done **Tickets** in that Session's project folder. A Ticket is a Markdown file in the project's ticket folder (by default `.scratch/*/issues/*.md`, the layout this repo uses). The list shows each Ticket's number, title, status and checklist progress, and tapping one shows its full text. Sessions from every supported **Harness** (Claude Code and Codex) get this option.

**Blocked by:** #14

**Status:** ready-for-agent

**GitHub:** —

- [ ] Every Session card on the phone and in the Pad has a "Pending tickets" action
- [ ] The list is read from the Session's project folder on every request and is never cached, so newly added or closed Tickets show up on refresh
- [ ] A Ticket is pending unless its `**Status:**` line is `done`, `closed` or `wontfix` (case-insensitive); a Ticket with no Status line counts as pending
- [ ] Each entry shows number, title (from the first heading), status, and checklist progress (for example `2/6`)
- [ ] Entries are ordered by ticket number, and "Blocked by" Tickets that are still pending are marked as blocked
- [ ] Tapping a Ticket shows its full Markdown, rendered read-only
- [ ] A project with no ticket folder or no pending Tickets shows a clear empty message
- [ ] The ticket folder pattern can be changed in the Pad, and the change persists in config
- [ ] The phone requests Tickets by Session ID only. The Core resolves the project folder, so a phone can never read arbitrary paths, and a Session outside the phone's **View** is rejected
- [ ] Covered over real HTTP with a temp project folder of fixture Tickets (pending, done, without a Status line, blocked)
