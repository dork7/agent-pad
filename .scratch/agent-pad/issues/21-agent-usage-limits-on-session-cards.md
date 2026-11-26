# 21: Remaining usage limits on Session cards

**What to build:** Each **Session** card, on the phone and in the Console, shows how much of its agent's usage limits is left, so you can see which agent is close to being cut off before you hand it more work (for example from the "Pending tickets" list in #19). The figures are the harness's own: for Claude Code, the 5-hour and weekly limits and context-window use; for Codex, its primary and weekly limits. Each shows when it resets. A harness that doesn't report a figure shows nothing for it, never a guess.

**Blocked by:** #14, ticket 19 (no GitHub issue yet)

**Status:** triage

**GitHub:** —

- [ ] Verify from each harness's docs where limit data is available locally (Claude Code: the status line JSON's rate limit and context fields, or hook input; Codex: its status data or rollout files) and record the sources in code, like the harness registry does
- [ ] Each Session card shows the percent used and reset time for every limit its harness reports, plus context-window use where available
- [ ] Limits near exhaustion are highlighted (for example 80% used or more), and a limit that is hit shows when it resets
- [ ] The ticket list from #19 shows the Session's remaining limit next to the "Implement" button from #20, so you don't start a ticket on an agent that is about to run out
- [ ] Figures refresh live as new data arrives, and stale figures (older than the last reset) are hidden
- [ ] Nothing is sent off the computer: data is read locally and served only over the existing phone and Console channels
- [ ] Tested with fixture status and limit payloads for Claude Code and Codex, including missing fields
