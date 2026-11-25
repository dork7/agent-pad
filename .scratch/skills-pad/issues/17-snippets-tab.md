# 17: Snippets tab

**What to build:** Your own non-skill text (common prompts, file paths, commit-message templates) saved in the Pad and pasted from a dedicated Snippets tab on the phone. Snippets reuse the existing tab, **View** and **Profile** behavior, so a **Phone** can be given or denied the Snippets chip like a **Harness**.

**Blocked by:** #10, #12

**Status:** ready-for-agent

**GitHub:** #18

- [ ] Snippets are created, edited, reordered and deleted in the Pad and persist across restarts
- [ ] A Snippets chip can be dragged onto a phone; phones without it see no Snippets tab
- [ ] Search on the phone covers snippet titles and text
- [ ] Tapping a snippet pastes its text exactly as saved (no added trailing space) using the normal paste flow
- [ ] The phone sends a snippet ID only; unknown IDs and snippets outside the phone's View are rejected
- [ ] Snippets chip assignments are captured and restored by Profiles
