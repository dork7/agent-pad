# 20: "Implement" command on pending tickets

**What to build:** Each pending **Ticket** shown on the phone or in the Pad (from #19) has an "Implement" button. Tapping it **Pastes** an implement **Invocation** for that Ticket, followed by Enter, so the agent at the cursor starts building it. The text comes from a per-**Harness** template, editable in the Pad. The default is `Implement the ticket at <ticket path> ` for every Harness. Ticket text is never sent by the phone, only IDs.

**Blocked by:** #5, #16, ticket 19 (no GitHub issue yet)

**Status:** in-progress (phone UI done against the endpoint contract)

**GitHub:** —

- [ ] Every pending Ticket on the phone and in the Pad shows an "Implement" button; Tickets that are blocked by a still-pending Ticket show it disabled, with the reason
  - [x] The phone shows Implement in the list and detail. Blocked tickets have a disabled button and a visible reason.
- [ ] Tapping it pastes the Harness's implement template with the Ticket's path filled in, then presses Enter, through the normal serialized paste flow (Pad hides first, clipboard restored)
- [ ] The template is taken from the Harness of the Session the list was opened from, and is editable and persisted in the Pad
- [ ] The phone sends a Session ID and Ticket ID only; unknown IDs, Tickets outside that Session's project and Sessions outside the phone's View are rejected
  - [x] The phone sends a bodyless POST with the two encoded IDs and displays server errors.
- [ ] The Pad warns once that the paste lands in whichever app has focus, so the user should focus that Session's terminal first
  - [x] The phone shows this note before the first Implement and lets the user cancel or confirm.
- [x] A brief "Sent to agent" confirmation appears on the phone
- [ ] Tested with the recording Paster: exact text, Enter after the text, and rejection of foreign IDs
