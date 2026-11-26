# 29: HUD hover panel and click to jump to a session

**What to build:** Hovering a logo on the **HUD** opens a panel listing that harness's **Sessions**, those needing you first, each with project, status and elapsed time. Clicking a session raises its window using the same matching as the phone's paste-into-session ticket.

**Blocked by:** #26, #22

**Status:** ready-for-agent

**GitHub:** #27

- [ ] Hover opens the panel; moving away closes it; keyboard focus reaches it too
- [ ] Sessions are ordered Blocked, Stalled, Running, Done, and each shows project, status and elapsed time
- [ ] Clicking a session raises its window, or says clearly that the window was not found
- [ ] The panel shows quotas and token figures once ticket 30 provides them
- [ ] Window raising reuses the existing replaceable interface, so tests use the recording fake
