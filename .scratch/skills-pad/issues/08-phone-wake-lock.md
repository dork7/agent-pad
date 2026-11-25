# 08: Phone wake lock

**What to build:** While the phone page is open the screen stays awake until the phone is locked or the page is left. Use the Screen Wake Lock API where available and the hidden-video fallback over plain HTTP.

**Blocked by:** #5

**Status:** ready-for-agent

**GitHub:** #9

- [ ] Screen does not sleep while the page is open and visible on real iOS and Android phones
- [ ] The lock releases when the page is hidden, the phone is locked, or the page is closed
- [ ] It re-acquires when returning to the page
- [ ] The page sends a heartbeat every 30 seconds that keeps sharing alive
