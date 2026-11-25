# 09: Added Skill Directories, custom harnesses, editable templates, and saved settings

**What to build:** Add any folder as a **Skill Directory**, assign it to an existing **Harness** or create a custom one (name plus Invocation template), and edit any harness's template. All settings persist in the OS config JSON.

**Blocked by:** #3, #6

**Status:** in-progress (Core done; Pad settings UI pending)

**GitHub:** #10

- [x] Skills from an added directory appear under the chosen harness on the phone
- [x] A custom harness gets its own tab and chip with its template applied
- [x] Editing a template changes the pasted text on the next tap
- [ ] Directories, custom harnesses, templates, hotkey and network override survive a restart
- [x] Skills themselves are never cached

**Notes:** Core commands: `add_directory(path, harness)`, `remove_directory`, `list_directories`, `add_custom_harness(name, template)`, `remove_custom_harness` (refused while directories are assigned), `set_template`, `reset_template`, `list_templates`, `set_sharing_address_override` and `sharing_address_override`. All of these are saved in `settings.json`. Templates must contain `{name}`. Skills carry `directory` (the added directory's ID, or null). Directories, custom harnesses, templates and the network override survive a restart (tested). The hotkey is saved by the shell in its own `config.json`. The restart criterion stays unticked until the Pad UI exists.
