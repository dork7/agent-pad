# 19: Pending tickets for a session's project

**What to build:** Tapping any **Session** card, on the phone or in the Pad, offers "Pending tickets". This opens a list of the not-yet-done **Tickets** in that Session's project folder. A Ticket is a Markdown file in the project's ticket folder (by default `.scratch/*/issues/*.md`, the layout this repo uses). The list shows each Ticket's number, title, status and checklist progress, and tapping one shows its full text. Sessions from every supported **Harness** (Claude Code and Codex) get this option.

**Blocked by:** #14

**Status:** done (manual checks pending)

**GitHub:** —

- [x] Every Pad Session card has a "Pending tickets" action.
- [ ] Every phone Session card offers "Pending tickets".
- [x] The list is read from the Session's project folder on every request and is never cached, so newly added or closed Tickets show up on refresh
- [x] A Ticket is pending unless its `**Status:**` value starts with `done` or is `closed` or `wontfix` (case-insensitive). A Ticket with no Status line counts as pending
- [x] Each Pad entry shows number, title (from the first heading), a status tag, and checklist progress (for example `2/6`)
  - [x] Core returns the number, title, status, and checklist counts.
- [x] The Pad shows the ticket's `**Status:**` value as written, such as `triage` or `ready-for-agent`. Known values get distinct colors, unknown values get a neutral tag, and a ticket with no Status line shows no tag
- [x] Entries are ordered by ticket number, and Tickets with a still-pending "Blocked by" Ticket have a blocked flag
- [x] Opening a Ticket in the Pad shows its full Markdown, rendered read-only
  - [x] Core returns the full Markdown for a valid Ticket ID.
- [x] In the Pad, a project with no ticket folder or no pending Tickets shows a clear empty message
  - [x] Core returns an empty list in both cases.
- [x] The ticket folder pattern can be changed in the Pad, and the change persists in config
  - [x] Core validates and persists changes to the pattern.
- [ ] The phone requests Tickets by Session ID only. The Core resolves the project folder, so a phone can never read arbitrary paths, and a Session outside the phone's **View** is rejected
  - [x] Core resolves list requests by Session ID and detail requests by Session ID and Ticket ID. Core enforces the phone's View.
- [x] Covered over real HTTP with a temp project folder of fixture Tickets (pending, done, without a Status line, blocked)

## Notes

- The local API and paired phone sharing server both expose `GET /sessions/<session_id>/tickets` and `GET /sessions/<session_id>/tickets/<ticket_id>`. URL-encode each ID as one path segment. Ticket IDs are project-relative paths that Core resolves again against the Session and current pattern. Core enforces the phone's View on these routes.
- The Pad can call `Core::pending_tickets(session_id)` and `Core::ticket(session_id, ticket_id)`. The list returns summaries. The detail response includes the summary fields at the top level plus `markdown`. The `blocked_by` field includes the original metadata value in `raw` and resolved local numbers in `ticket_numbers`.
- The Pad can call `Core::ticket_pattern()` for the saved override and `Core::set_ticket_pattern(Some(pattern))` to save a change. Pass `None` to restore the default `.scratch/*/issues/*.md`. Use forward slashes and `*` within a path segment.

The phone ticket UI is on `build/specs` (`6be2d6e`). Core tracks sessions by harness and ID, but ticket APIs resolve by ID alone. The Pad rejects ticket operations when two harnesses report the same ID so it does not pick the wrong project.

## Manual checks pending

- [ ] Open a project with a mix of pending, blocked, and closed tickets. Check tags, progress, blocker text, and read-only Markdown. Add or close a ticket on disk and use Refresh. Check the empty project case.
- [ ] Change the ticket pattern, restart the Pad, verify the saved pattern, then reset it. Exercise the phone flow on the integrated build.
