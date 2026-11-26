# 26: In-page alerts when an agent becomes Blocked or Done

**What to build:** While the phone page is open, a session turning Blocked or Done raises a banner, a short chime and a flash on its card or tile, and vibrates on Android. Over plain HTTP the page cannot receive push notifications, so alerts work only while the page is open; this is stated in the page. A mute toggle in the header is remembered per phone.

**Blocked by:** #20, #9

**Status:** ready-for-agent

**GitHub:** #24

- [ ] A banner naming the project and new status appears on a Blocked or Done transition, and dismisses on tap or after a few seconds
- [ ] A short chime plays after the first tap on the page (browsers require a gesture); a mute toggle silences chime and vibration
- [ ] The card or tile flashes once; Android vibrates, iOS does nothing and does not error
- [ ] No alert fires for the initial list load or for sessions that are already in that status
- [ ] The mute setting is stored per phone and survives reloads
- [ ] The page tells the user alerts need it open and awake
