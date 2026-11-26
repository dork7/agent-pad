# 30: Quotas, burn rates and 24-hour token stats

**What to build:** Extend the usage-limits work so each harness reports its quotas with reset times and burn rates, plus token use over the last 24 hours. The figures appear on the **HUD** hover panel, on Session cards in the **Console** and on the phone. A harness that does not report a figure shows nothing, never a guess.

**Blocked by:** ticket 21 (no GitHub issue yet), #14, #15

**Status:** ready-for-agent

**GitHub:** #28

- [ ] Data sources for Claude Code and Codex are verified from their docs and recorded in code, building on ticket 21's research
- [ ] Each harness shows percent used and reset time per quota, and a burn rate where it can be computed
- [ ] Token use over the last 24 hours is shown as bars, per harness
- [ ] Quotas near exhaustion are highlighted using the 70% and 90% thresholds
- [ ] Stale figures older than the last reset are hidden
- [ ] Nothing leaves the computer; data is read locally
- [ ] Tested with fixture payloads including missing fields
