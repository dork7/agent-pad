# 16: Argument presets on skills

**What to build:** Attach saved suffixes to a **Skill**, such as `high` for a review skill or a standard prompt. A skill with presets shows each as a one-tap button on the phone, and tapping pastes the full line (**Invocation** plus the preset).

**Blocked by:** #5, #10

**Status:** in-progress (Core done; phone and Pad UI pending)

**GitHub:** #17

- [ ] Presets are created, edited and deleted in the Pad and are saved in settings
- [ ] A skill with presets shows them on the phone beside the plain tap; plain tap still pastes the bare Invocation
- [x] The pasted text is the Invocation, then the preset text, with a trailing space
- [x] The phone sends a skill ID and a preset ID only; arbitrary text is rejected by the Core
- [ ] Deleting a preset removes it from phones live
- [x] Presets belong to a skill by harness and name, so they survive rescans

**Notes:** Skills carry `presets: [{id, text}]` (empty by default). `POST /paste` accepts an optional `preset` ID and pastes `<invocation> <preset> `. Core commands: `list_presets(skill_id)`, `add_preset`, `update_preset` and `delete_preset`, saved in settings. Deleting a preset removes it on the phone's next reload. Still to build: phone buttons and the Pad editor.
