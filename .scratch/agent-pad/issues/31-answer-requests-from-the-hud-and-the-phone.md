# 31: Answer requests from the HUD and the phone

**What to build:** When a session is Blocked on a **Request**, the **HUD** and the phone's Blocked card offer Approve, Deny and Always allow for permission requests, and let you answer a question the agent asked. It is opt-in per harness. The agent waits for the answer up to a timeout, then falls back to its own normal prompt. There is no automatic approval.

**Blocked by:** #26, #14, #15

**Status:** ready-for-agent

**GitHub:** #29

- [ ] Verify from each harness's hook docs that a permission hook can return a decision, and record how each harness reports questions
- [ ] Enabling it for a harness is explicit, shows what changes in its config, and can be undone in one click
- [ ] Approve, Deny and Always allow reach the right agent and the session leaves Blocked
- [ ] If nobody answers before the timeout, the agent's own prompt appears as if Agent Pad were not there
- [ ] The phone and the HUD can answer; whichever answers first wins and the other updates
- [ ] A phone can only answer requests for sessions in its View
- [ ] Tested with fixture hook payloads and a fake clock for the timeout
