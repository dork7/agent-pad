# 04: Phone page: tabs, search, and tap-to-paste on macOS

**What to build:** First end-to-end demo. Scan the QR code, see a tab per **Harness** with search, tap a **Skill**, and its **Invocation** is **Pasted** at the cursor of the focused Mac app. The Pad hides first if visible, the clipboard is restored, and pastes are serialized.

**Blocked by:** #3, #4

**Status:** done (manual checks pending)

**GitHub:** #5

- [x] The embedded phone page loads from the scanned URL and shows a tab per discovered harness
- [x] Search filters by name and description
- [x] Tapping a skill sends its ID only; the Core builds the text, so arbitrary text is never accepted
- [x] If the Pad is focused it hides, focus returns, and then the paste happens
- [ ] The previous clipboard contents are restored after the paste
- [x] Concurrent taps are pasted one at a time in arrival order (tested with the recording Paster)
- [x] A brief "Pasted" confirmation appears on the phone

**Notes:** The phone page is embedded at `crates/core/src/phone.html` and served by the sharing server, along with `GET /harnesses` and `POST /paste` (`{"id": ...}`; unknown IDs return 404, anything else 400). Every paste, from a phone or from `Core::paste_skill`, goes through one queue. The shell registers its window with `Core::set_pad_window` so the Pad hides first. The real paster is `SystemPaster` (arboard + enigo). It restores text clipboard contents only; an image on the clipboard is lost.

**Manual checks:** run `cargo run -p skills-pad-core --example share`, open the printed URL on a phone, focus a text field on the Mac and tap a skill. Confirm the text lands and the previous clipboard text comes back. The terminal needs Accessibility permission. The clipboard-restore criterion is still unticked until this check is done.
