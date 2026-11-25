# 08: Phone wake lock

**What to build:** While the phone page is open the screen stays awake until the phone is locked or the page is left. Use the Screen Wake Lock API where available and the hidden-video fallback over plain HTTP.

**Blocked by:** #5

**Status:** done (manual checks pending)

**GitHub:** #9

- [ ] Screen does not sleep while the page is open and visible on real iOS and Android phones
- [x] The lock releases when the page is hidden, the phone is locked, or the page is closed
- [x] It re-acquires when returning to the page
- [x] The page sends a heartbeat every 30 seconds that keeps sharing alive

**Notes:** The page uses the Screen Wake Lock API in a secure context. Otherwise, which is the usual case over plain LAN HTTP, it plays a muted, looping 2-second video inlined as a data URI (iOS won't play served video without range requests). The video is paused when the page is hidden or left, and replayed on return and on each tap until it runs.

**Manual checks:** on a real iPhone and a real Android phone, leave the page open untouched beyond the screen-lock timeout and confirm the screen stays on. Then lock the phone, return, and confirm it stays on again.
