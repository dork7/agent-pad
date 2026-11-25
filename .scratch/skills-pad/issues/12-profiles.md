# 12: Profiles

**What to build:** Save the arrangement of every phone's **View** as a named **Profile**, then switch, rename and delete Profiles from a dropdown in the Pad.

**Blocked by:** #12

**Status:** done (manual checks pending)

**GitHub:** #13

- [x] Saving captures every known phone's View by device ID
- [x] Switching applies all Views at once, live on connected phones
- [x] A profile entry for a phone that is no longer known is skipped without error
- [x] Profiles persist across restarts
- [x] The Pad toolbar selects the active Profile and offers Save as, Rename, and Delete inline.

**Notes:** Core commands: `list_profiles()` returns `(profiles, active)`; plus `save_profile(name)` (replaces a Profile with the same name), `switch_profile`, `rename_profile` and `delete_profile`. Switching skips unknown device IDs, leaves Phones the Profile doesn't mention unchanged, and drops chips that no longer exist. Views apply live through the phone page's `/view` poll. The toolbar switches Profiles and refreshes the Pad’s phone cards immediately.

## Manual checks pending

- [ ] Save two Profiles with distinct Views across paired phones. Switch between them and confirm the phones update live. Rename and delete a Profile, then restart the Pad and check the remaining names and active selection.
