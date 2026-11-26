# 23: Home as the running-agents list, with a Session screen

**What to build:** The phone's **Home** is the list of running **Sessions** with their status, sorted by urgency (Blocked, then Stalled, Running, Done). Opening a session shows its **Session screen**: a header with project folder, status, elapsed time and last event, then the skill picker for that session's harness (its tab plus Shared and Snippets), with search and the phone's **View** applied. A permanent "All skills" entry opens the skill picker with no session. With no sessions, Home explains that agents appear when they start and that sessions opened before the hooks were installed show up after a restart; with hooks not installed it says how to install them.

**Blocked by:** #20, #5

**Status:** ready-for-agent

**GitHub:** #21

- [ ] Home lists sessions that the phone's View allows, ordered by urgency
- [ ] Tapping a session opens its Session screen with the header and that harness's skill picker
- [ ] A phone can only open sessions for harnesses in its View
- [ ] All skills opens the pad without a session and pastes at the cursor as before
- [ ] Empty state with no sessions, and the "status is off" state without hooks, both explain what to do and keep All skills one tap away
- [ ] A back control returns to Home and keeps the scroll position
- [ ] Covered by Core tests for session ordering and View filtering, and a manual check on a real phone
