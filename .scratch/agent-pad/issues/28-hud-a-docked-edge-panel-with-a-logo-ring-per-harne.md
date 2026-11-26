# 28: HUD: a docked edge panel with a logo ring per harness

**What to build:** A small always-on-top **HUD** docked to a screen edge and draggable, showing one logo per **Harness** that has sessions. Each logo sits in a ring coloured by that harness's most used quota (green under 70% used, yellow from 70%, red from 90%), and the logo animates by the most urgent **Status** among its sessions. It is on by default and can be switched off in the **Console**. The Console itself stays a normal window that is not always on top.

**Blocked by:** #20

**Status:** done (manual checks pending)

**GitHub:** #26

- [ ] The HUD stays above other windows, can be dragged to any screen edge and remembers its place
- [x] One logo per harness with live sessions, using the animated icons from the status ticket
- [x] The ring colour follows the quota thresholds when quota data exists (ticket 30) and is neutral until it does
- [x] The logo animation reflects the most urgent status of that harness's sessions
- [x] A toggle in the Console shows or hides the HUD, and the choice persists
- [ ] Works on macOS and Windows; the notch mode is not part of this ticket
