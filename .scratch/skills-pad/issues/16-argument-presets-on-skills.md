# 16: Argument presets on skills

**What to build:** Attach saved suffixes to a **Skill**, such as `high` for a review skill or a standard prompt. A skill with presets shows each as a one-tap button on the phone, and tapping pastes the full line (**Invocation** plus the preset).

**Blocked by:** #5, #10

**Status:** done (manual checks pending)

**GitHub:** #17

- [x] Presets are created, edited and deleted in the Pad and are saved in settings
- [ ] A skill with presets shows them on the phone beside the plain tap; plain tap still pastes the bare Invocation
- [x] The pasted text is the Invocation, then the preset text, with a trailing space
- [x] The phone sends a skill ID and a preset ID only; arbitrary text is rejected by the Core
- [ ] Deleting a preset removes it from phones live
- [x] Presets belong to a skill by harness and name, so they survive rescans

**Notes:** Skills carry `presets: [{id, text}]` (empty by default). `POST /paste` accepts an optional `preset` ID and pastes `<invocation> <preset> `. Core commands: `list_presets(skill_id)`, `add_preset`, `update_preset` and `delete_preset`, saved in settings. Deleting a preset removes it on the phone's next reload. The Pad searches discovered skills and manages argument presets for the selected skill.

Phone preset buttons and live inventory refresh are implemented on `build/specs` (`cededc6`). The phone-side checkboxes in this branch track its current source tree.

## Manual checks pending

- [ ] Add, edit, and delete a preset in the Pad and check persistence after restart. On the phone UI from build/specs, verify the bare invocation, the preset invocation, and that a deleted preset disappears after reload.
