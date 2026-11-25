# 12: Profiles

**What to build:** Save the arrangement of every phone's **View** as a named **Profile**, then switch, rename and delete Profiles from a dropdown in the Pad.

**Blocked by:** #12

**Status:** in-progress (Core done; Pad dropdown pending)

**GitHub:** #13

- [x] Saving captures every known phone's View by device ID
- [x] Switching applies all Views at once, live on connected phones
- [x] A profile entry for a phone that is no longer known is skipped without error
- [x] Profiles persist across restarts

**Notes:** Core commands: `list_profiles()` returns `(profiles, active)`; plus `save_profile(name)` (replaces a Profile with the same name), `switch_profile`, `rename_profile` and `delete_profile`. Switching skips unknown device IDs, leaves Phones the Profile doesn't mention unchanged, and drops chips that no longer exist. Views apply live through the phone page's `/view` poll. The Pad dropdown is still to build.
