# 18: Phone polish: light and dark themes, haptics, one-handed layout

**What to build:** Make the phone page comfortable to tap many times a day: it follows the phone's light or dark setting, gives tactile feedback on a tap, and places search and the most-used controls within thumb reach.

**Blocked by:** #5

**Status:** done (manual checks pending)

**GitHub:** #19

- [x] The page follows the phone's light and dark setting and is readable in both
- [x] A tap gives haptic feedback where the browser allows it (Android vibration; iOS Safari does not support it, so it degrades silently)
- [x] The layout works one-handed: tabs and search at the bottom, large tap targets
- [x] No horizontal scrolling at common phone widths
- [ ] Verified on a real iPhone and a real Android phone

**Notes:** Rows show the invocation with a description capped at two lines, and tap targets are at least 44px tall. Tabs and search sit in a bottom dock that respects the safe area. Checked with Chrome mobile emulation at 390px and 360px in both themes: no horizontal scroll.

**Manual checks:** verify on a real iPhone and a real Android phone (last criterion).
