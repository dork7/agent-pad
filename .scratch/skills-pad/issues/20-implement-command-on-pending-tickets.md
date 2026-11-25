# 20: "Implement" command on pending tickets

**What to build:** Each pending **Ticket** shown on the phone or in the Pad (from #19) has an "Implement" button. Tapping it **Pastes** an implement **Invocation** for that Ticket, followed by Enter, so the agent at the cursor starts building it. The text comes from a per-**Harness** template, editable in the Pad. The default is `Implement the ticket at <ticket path> ` for every Harness. Ticket text is never sent by the phone, only IDs.

**Blocked by:** #5, #16, ticket 19 (no GitHub issue yet)

**Status:** done (manual checks pending)

**GitHub:** —

- [x] Every pending Ticket in the Pad shows an "Implement" button. Tickets blocked by a still-pending Ticket show it disabled, with the reason.
- [x] The phone shows the same Implement action and blocker reason.
- [x] Tapping it pastes the Harness's implement template with the Ticket's path filled in, then presses Enter, through the normal serialized paste flow (Pad hides first, clipboard restored)
- [x] The template is taken from the Harness of the Session the list was opened from, and is editable and persisted in the Pad
- [x] The phone sends a Session ID and Ticket ID only; unknown IDs, Tickets outside that Session's project and Sessions outside the phone's View are rejected
- [x] The Pad warns once that the paste lands in whichever app has focus, so the user should focus that Session's terminal first
- [x] A brief "Sent to agent" confirmation appears on the phone
- [x] Tested with the recording Paster: exact text, Enter after the text, and rejection of foreign IDs

**Notes:** `POST /sessions/<sid>/tickets/<tid>/implement` exists on the local API and the phone sharing server. It returns 200 `{"pasted"}`, 409 if the ticket is blocked or done, and 404 if the session or ticket is unknown. Core commands: `implement_ticket`, `implement_template(harness)` and `set_implement_template(harness, Some|None)`; templates must contain `{path}`. The text goes through the normal paste queue and is always followed by Enter. The sharing server rejects sessions outside the phone's View. The Pad edits and resets a separate template for Claude Code and Codex.

The phone action and confirmation are on `build/specs` (`6be2d6e`).

## Manual checks pending

- [ ] In a disposable project, focus the intended terminal, open the Pad, and try Implement. Confirm the first-use warning can be canceled, then confirm the command pastes into that terminal with Enter and that the clipboard is restored.
- [ ] Save different templates for Claude Code and Codex, verify each uses its own template, then reset both. Verify a blocked or newly closed ticket cannot send, and check the confirmation on the integrated phone build.
