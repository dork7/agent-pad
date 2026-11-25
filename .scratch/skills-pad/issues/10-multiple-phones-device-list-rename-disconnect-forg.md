# 10: Multiple phones: device list, rename, disconnect, forget

**What to build:** Several **Phones** can pair at once. Each keeps a long-lived device ID, is auto-named from its browser and renamable, and appears in the Pad with an online dot and last-seen time. Disconnect revokes a phone until the next scan; Forget deletes its saved state.

**Blocked by:** #4, #6

**Status:** ready-for-agent

**GitHub:** #11

- [ ] Two phones can use the same sharing session simultaneously
- [ ] A known phone is recognized after a new sharing session (new token, same device ID)
- [ ] Disconnect makes a phone's requests fail until it scans again
- [ ] Forget removes the phone and its saved View
- [ ] The Pad shows name, online state and last seen for each phone
