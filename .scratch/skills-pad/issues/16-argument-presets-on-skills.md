# 16: Argument presets on skills

**What to build:** Attach saved suffixes to a **Skill**, such as `high` for a review skill or a standard prompt. A skill with presets shows each as a one-tap button on the phone, and tapping pastes the full line (**Invocation** plus the preset).

**Blocked by:** #5, #10

**Status:** ready-for-agent

**GitHub:** #17

- [ ] Presets are created, edited and deleted in the Pad and are saved in settings
- [ ] A skill with presets shows them on the phone beside the plain tap; plain tap still pastes the bare Invocation
- [ ] The pasted text is the Invocation, then the preset text, with a trailing space
- [ ] The phone sends a skill ID and a preset ID only; arbitrary text is rejected by the Core
- [ ] Deleting a preset removes it from phones live
- [ ] Presets belong to a skill by harness and name, so they survive rescans
