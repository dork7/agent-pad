# 11: Views: drag harness and directory chips onto phones

**What to build:** Each phone card in the **Console** accepts dragged **Harness** and **Skill Directory** chips that define that phone's **View**, applied live. New phones see everything by default.

**Blocked by:** #3, #11

**Status:** done (manual checks pending)

**GitHub:** #12

- [x] Dropping a harness chip gives that phone all of the harness's skills
- [x] Dropping a directory chip gives only that directory's skills
- [x] The × on a chip removes it and the phone's list updates live
- [x] A phone with nothing assigned shows an empty message
- [x] A phone cannot paste a skill outside its View (rejected by the Core)
- [x] A phone's View survives restarts

**Notes:** A View is `null` (everything, the default for new Phones; it also covers harnesses added later) or a list of `h:<harness>` and `d:<directory>` chips. Core commands: `list_chips`, `set_phone_view`, `add_chip_to_phone` and `remove_chip_from_phone`. Removing a chip from "everything" expands the View to every current chip except that one. The phone's `/skills`, `/harnesses` and `/paste` honor the View (403 outside it). The phone page polls `GET /view` every 3 s and reloads when the View changes, so updates apply live. Removing an added directory drops its chip from every View. The Console exposes chip removal, drag and drop, and a keyboard-accessible Add control.

Core follow-up: `PhoneError` lacks `Display` in this checkout. The Console uses its existing messages for those variants. A harness chip includes all of its directories, so removing a directory chip leaves skills still covered by a harness chip visible.

**Manual checks pending**

- Pair two phones and verify that a new phone shows Everything. Use × and drag both chip types onto one phone. Verify that its page updates within the View polling interval and the other phone keeps its View.
- Remove all chips and verify the phone's empty state. Use the keyboard to add a chip back and restore Everything. Quit and relaunch with an explicit View and verify that the card and phone page agree.
- Verify the online dot, last-seen text, rename, disconnect, and forget controls with a paired phone.
