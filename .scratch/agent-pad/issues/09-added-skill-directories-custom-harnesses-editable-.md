# 09: Added Skill Directories, custom harnesses, editable templates, and saved settings

**What to build:** Add any folder as a **Skill Directory**, assign it to an existing **Harness** or create a custom one (name plus Invocation template), and edit any harness's template. All settings persist in the OS config JSON.

**Blocked by:** #3, #6

**Status:** done (manual checks pending)

**GitHub:** #10

- [x] Skills from an added directory appear under the chosen harness on the phone
- [x] A custom harness gets its own tab and chip with its template applied
- [x] Editing a template changes the pasted text on the next tap
- [x] Directories, custom harnesses, templates, hotkey and network override survive a restart
- [x] Skills themselves are never cached

**Notes:** Core commands: `add_directory(path, harness)`, `remove_directory`, `list_directories`, `add_custom_harness(name, template)`, `remove_custom_harness` (refused while directories are assigned), `set_template`, `reset_template`, `list_templates`, `set_sharing_address_override` and `sharing_address_override`. All of these are saved in `settings.json`. Templates must contain `{name}`. Skills carry `directory` (the added directory's ID, or null). Directories, custom harnesses, templates and the network override survive a restart (tested). The hotkey is saved by the shell in its own `config.json`. The Console exposes these settings and uses the native folder picker.

Core follow-up: `sharing_address` returns the effective address, so the Console can show the automatic address only while the override is cleared. The override setter logs save errors and returns no result. The Console checks the saved value after each change to report a failed update.

**Manual checks pending**

- Choose a folder in the native picker, assign it to a built-in harness and a second folder to a custom harness, and verify the counts and phone tabs. Cancel the picker and confirm the list stays the same.
- Try an invalid template and removing a custom harness with a folder assigned. Verify the inline errors, then save a valid template and reset a built-in template. Tap a skill from a phone and verify the pasted invocation after each change.
- Save a directory, custom harness, template edit, hotkey, and network override. Quit and relaunch the Console. Verify the saved values, exercise the hotkey, start sharing on the override, then clear it and verify the next share uses the automatic address.
