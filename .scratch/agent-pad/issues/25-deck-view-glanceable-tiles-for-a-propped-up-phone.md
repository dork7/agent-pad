# 25: Deck view: glanceable tiles for a propped-up phone

**What to build:** Home gets a List and Deck toggle. Deck is a full-screen grid of large tiles, one per session, showing the harness icon, project name, status colour and elapsed time, readable from across a desk. Blocked tiles grow and pulse. Tapping a tile opens its Session screen. Deck keeps the screen awake with the existing wake lock.

**Blocked by:** #21, #9

**Status:** ready-for-agent

**GitHub:** #23

- [ ] A toggle at the top of Home switches between List and Deck and is remembered per phone
- [ ] Deck tiles show icon, project, status and elapsed time, sorted by urgency
- [ ] Blocked tiles are visibly larger and animate; the others stay calm
- [ ] Tapping a tile opens the Session screen
- [ ] The screen stays awake in Deck while the page is visible, using the existing wake lock and fallback
- [ ] Layout works from small phones to tablets, in light and dark themes
