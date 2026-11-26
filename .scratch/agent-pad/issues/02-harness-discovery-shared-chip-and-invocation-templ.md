# 02: Harness discovery, Shared chip, and Invocation templates

**What to build:** The Core discovers **Harnesses** from a registry (Claude Code, Codex, Cursor, Gemini CLI, OpenCode, Copilot). A harness appears only when skills are found for it, `~/.agents` appears as a separate Shared chip, and every skill carries the harness-specific **Invocation** (trailing space included). Verify the real typed forms (Codex `$name` and the others) against each harness's docs as part of this ticket.

**Blocked by:** #2

**Status:** done

**GitHub:** #3

- [x] Harnesses with no skills found are not listed
- [x] The shared agents folder is its own chip, not attributed to a harness
- [x] Claude skills yield `/name` (or `/plugin:skill`) and Codex skills yield `$name`, each followed by a space
- [x] Harnesses without a documented typed form use the plain-text default template
- [x] Verified invocation forms are recorded in the registry with their doc sources

**Notes:** Verified against each harness's docs (sources are in `crates/core/src/harness.rs`). Claude `/name`, Codex `$name`, Cursor `/name`, Copilot `/name`, OpenCode `@name`. Gemini CLI has no typed form (the model activates skills itself), so it uses the plain-text default, as does the Shared chip. Codex's documented global folder is `~/.agents/skills` (the Shared chip). `~/.codex/skills` is still scanned but is only supported by user reports. The local API adds `GET /harnesses`, and every skill carries an `invocation`.
