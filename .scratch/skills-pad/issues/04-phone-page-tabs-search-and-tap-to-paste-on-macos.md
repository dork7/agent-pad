# 04: Phone page: tabs, search, and tap-to-paste on macOS

**What to build:** First end-to-end demo. Scan the QR code, see a tab per **Harness** with search, tap a **Skill**, and its **Invocation** is **Pasted** at the cursor of the focused Mac app. The Pad hides first if visible, the clipboard is restored, and pastes are serialized.

**Blocked by:** #3, #4

**Status:** ready-for-agent

**GitHub:** #5

- [ ] The embedded phone page loads from the scanned URL and shows a tab per discovered harness
- [ ] Search filters by name and description
- [ ] Tapping a skill sends its ID only; the Core builds the text, so arbitrary text is never accepted
- [ ] If the Pad is focused it hides, focus returns, and then the paste happens
- [ ] The previous clipboard contents are restored after the paste
- [ ] Concurrent taps are pasted one at a time in arrival order (tested with the recording Paster)
- [ ] A brief "Pasted" confirmation appears on the phone
