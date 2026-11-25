# 02: Harness discovery, Shared chip, and Invocation templates

**What to build:** The Core discovers **Harnesses** from a registry (Claude Code, Codex, Cursor, Gemini CLI, OpenCode, Copilot). A harness appears only when skills are found for it, `~/.agents` appears as a separate Shared chip, and every skill carries the harness-specific **Invocation** (trailing space included). Verify the real typed forms (Codex `$name` and the others) against each harness's docs as part of this ticket.

**Blocked by:** #2

**Status:** ready-for-agent

**GitHub:** #3

- [ ] Harnesses with no skills found are not listed
- [ ] The shared agents folder is its own chip, not attributed to a harness
- [ ] Claude skills yield `/name` (or `/plugin:skill`) and Codex skills yield `$name`, each followed by a space
- [ ] Harnesses without a documented typed form use the plain-text default template
- [ ] Verified invocation forms are recorded in the registry with their doc sources
