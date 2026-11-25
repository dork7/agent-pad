# 17: Snippets tab

**What to build:** Your own non-skill text (common prompts, file paths, commit-message templates) saved in the Pad and pasted from a dedicated Snippets tab on the phone. Snippets reuse the existing tab, **View** and **Profile** behavior, so a **Phone** can be given or denied the Snippets chip like a **Harness**.

**Blocked by:** #10, #12

**Status:** in-progress (Core and phone done; Pad editor in progress)

**GitHub:** #18

- [ ] Snippets are created, edited, reordered and deleted in the Pad and persist across restarts
- [ ] A Snippets chip can be dragged onto a phone; phones without it see no Snippets tab
- [x] Search on the phone covers snippet titles and text
- [x] Tapping a snippet pastes its text exactly as saved (no added trailing space) using the normal paste flow
- [x] The phone sends a snippet ID only; unknown IDs and snippets outside the phone's View are rejected
- [x] Snippets chip assignments are captured and restored by Profiles

**Notes:** Core commands: `list_snippets`, `add_snippet(title, text)`, `update_snippet`, `move_snippet(id, index)` and `delete_snippet`, saved in settings. The `snippets` chip works with Views and Profiles like any other chip, and a View of `null` includes it. Phone endpoints: `GET /snippets` (empty if the chip is missing) and `POST /paste-snippet {id}`, which pastes the text exactly as saved, with 403 outside the View and 404 for an unknown ID. Still to build: the phone Snippets tab with search, and the Pad editor.

Phone UI: presets appear as one-tap buttons under their skill. The Snippets tab appears when the phone has snippets, and search covers titles and text. Try it safely with `cargo run -p skills-pad-core --example demo_phone`.
