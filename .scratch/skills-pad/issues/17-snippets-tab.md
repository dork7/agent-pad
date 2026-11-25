# 17: Snippets tab

**What to build:** Your own non-skill text (common prompts, file paths, commit-message templates) saved in the Pad and pasted from a dedicated Snippets tab on the phone. Snippets reuse the existing tab, **View** and **Profile** behavior, so a **Phone** can be given or denied the Snippets chip like a **Harness**.

**Blocked by:** #10, #12

**Status:** done (manual checks pending)

**GitHub:** #18

- [x] Snippets are created, edited, reordered and deleted in the Pad and persist across restarts
- [x] In the Pad, the Snippets chip can be dragged or assigned to a phone.
- [x] Phones without the chip see no Snippets tab.
- [x] Search on the phone covers snippet titles and text
- [x] Tapping a snippet pastes its text exactly as saved (no added trailing space) using the normal paste flow
- [x] The phone sends a snippet ID only; unknown IDs and snippets outside the phone's View are rejected
- [x] Snippets chip assignments are captured and restored by Profiles

**Notes:** Core commands: `list_snippets`, `add_snippet(title, text)`, `update_snippet`, `move_snippet(id, index)` and `delete_snippet`, saved in settings. The `snippets` chip works with Views and Profiles like any other chip, and a View of `null` includes it. Phone endpoints: `GET /snippets` (empty if the chip is missing) and `POST /paste-snippet {id}`, which pastes the text exactly as saved, with 403 outside the View and 404 for an unknown ID. The Pad editor supports multiline text, drag and button reordering, and live refresh of the chip palette.

The phone Snippets tab and search are implemented on `build/specs` (`cededc6`). The phone-side checkboxes in this branch track its current source tree.

## Manual checks pending

- [ ] Create multiline snippets, edit them, reorder by drag and by buttons, and check order after restarting the Pad. Assign the Snippets chip with drag and keyboard controls.
- [ ] On the phone UI from build/specs, verify search, View filtering, and exact multiline paste.
