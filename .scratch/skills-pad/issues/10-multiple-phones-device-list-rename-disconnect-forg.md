# 10: Multiple phones: device list, rename, disconnect, forget

**What to build:** Several **Phones** can pair at once. Each keeps a long-lived device ID, is auto-named from its browser and renamable, and appears in the Pad with an online dot and last-seen time. Disconnect revokes a phone until the next scan; Forget deletes its saved state.

**Blocked by:** #4, #6

**Status:** in-progress (Core done; Pad phone cards pending the shell from ticket 05)

**GitHub:** #11

- [x] Two phones can use the same sharing session simultaneously
- [x] A known phone is recognized after a new sharing session (new token, same device ID)
- [x] Disconnect makes a phone's requests fail until it scans again
- [x] Forget removes the phone and its saved View
- [ ] The Pad shows name, online state and last seen for each phone

**Notes:** Each Phone gets a `skills_pad_device` cookie (one year) on its first authenticated request. Known Phones are saved under `phones` in `settings.json`. Core commands: `list_phones()` returns id, name, online, last_seen (unix seconds) and view; plus `rename_phone`, `disconnect_phone` and `forget_phone`. A Phone counts as online if it made a request in the last 75 s. Disconnect and Forget both refuse the Phone until it scans the QR again; Forget also deletes the record, so the Phone comes back as new. Cookies are tied to the computer's address, so a new LAN address makes a Phone look new. The last criterion (the Pad shows the phones) needs the Pad UI.
