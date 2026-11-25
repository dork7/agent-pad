## Problem Statement

I use several coding-agent **Harnesses** (Claude Code, Codex, Cursor and others), each with its own **Skills** spread across global folders, plugin caches and extra folders. Remembering exact skill names and typing their **Invocations** is slow, and I can't see at a glance which of my agent **Sessions** are working, waiting on me, or stuck. I want my phone to act as a remote picker and status board for my computer, without the phone ever being the place where text lands.

## Solution

Skills Pad is a Rust desktop app for macOS and Windows. A global hotkey or status-bar icon summons the **Pad**. From the Pad I choose "Open on phone", which starts a LAN server on demand and shows a QR code. Scanning it opens a mobile web page listing my skills in tabs per Harness. Tapping a skill **Pastes** its Invocation at the cursor of whatever app has focus on the computer. Each paired **Phone** has its own **View**, set by dragging Harness and Skill Directory chips onto its card in the Pad, and arrangements can be saved as **Profiles**. The phone page also shows live **Status** for each **Session** (working, needs input, idle, stuck, ended) for Claude Code and Codex.

## User Stories

1. As a developer, I want a global hotkey to show and hide the Pad, so that I can reach Skills Pad without hunting for a window.
2. As a developer, I want a status-bar icon with a menu, so that the app is reachable even when its window is hidden.
3. As a developer, I want the Pad to not stay on top of other windows, so that it never covers my work.
4. As a developer, I want to rebind the hotkey in the Pad, so that it doesn't clash with other shortcuts.
5. As a developer, I want the default hotkey to be Cmd+Option+S on macOS and Ctrl+Alt+S on Windows, so that it feels native on each OS.
6. As a developer, I want sharing to be off until I choose "Open on phone", so that nothing listens on my network unexpectedly.
7. As a developer, I want "Open on phone" to show a QR code and the address as text, so that I can scan it or type it.
8. As a developer, I want a fresh token each time sharing starts, so that an old QR code never works again.
9. As a developer, I want the app to pick my primary LAN address automatically, so that the QR works without configuration.
10. As a developer with VPN or Docker adapters, I want to override the chosen network address, so that the QR points to the right interface.
11. As a developer, I want a random port each time, so that it never collides with another service.
12. As a Windows user, I want a clear explanation of the firewall prompt, so that I know to allow it.
13. As a developer, I want sharing to stop after 15 minutes without phone activity, so that the server isn't left open.
14. As a developer, I want an open phone page to count as activity, so that sharing doesn't stop while I'm using it.
15. As a developer, I want a "Stop sharing" button, so that I can end sharing immediately.
16. As a developer, I want the phone page to open in the phone's browser with no app install, so that it works on iOS and Android.
17. As a phone user, I want the first scan to remember me with a cookie, so that I don't need to re-scan during a session.
18. As a phone user, I want to search skills by name and description, so that I find one quickly.
19. As a phone user, I want tabs per Harness, so that I only see skills for the agent I'm using.
20. As a phone user, I want each skill shown with its name and one-line description, so that I can choose confidently.
21. As a phone user, I want one tap to paste a skill, so that using it is effortless.
22. As a phone user, I want a brief "Pasted" confirmation, so that I know the tap worked.
23. As a phone user, I want my screen to stay awake while the page is open, so that I don't have to keep unlocking.
24. As a phone user, I want the awake behavior to end when I lock the phone or leave the page, so that my battery isn't drained.
25. As a phone user over plain HTTP, I want the wake behavior to work via a fallback, so that it works without HTTPS.
26. As a developer, I want the skill list rescanned whenever the phone loads or refreshes, so that new installs show up immediately.
27. As a developer, I want global skills discovered by default, so that I have to configure nothing.
28. As a developer, I want Harnesses discovered by scanning, so that the app adapts to what I have installed.
29. As a developer, I want a Harness chip only when skills were found for it, so that empty Harnesses don't clutter the Pad.
30. As a developer, I want `~/.agents` shown as its own "Shared" chip, so that I'm not misled about which Harness owns it.
31. As a developer, I want multiple plugin versions de-duplicated to the newest, so that I don't see repeated skills.
32. As a developer, I want to add my own Skill Directories, so that team or project skills appear too.
33. As a developer, I want to assign an added directory to an existing Harness, so that its skills use the right Invocation.
34. As a developer, I want to create a custom Harness with a name and Invocation template, so that unsupported agents still work.
35. As a developer, I want Claude skills to paste as `/name` (or `/plugin:skill`), so that they run in Claude Code.
36. As a developer, I want Codex skills to paste as `$name`, so that they run in Codex.
37. As a developer, I want a plain-text default Invocation for other Harnesses, so that something sensible is pasted.
38. As a developer, I want to edit each Harness's Invocation template, so that I can correct or customize it.
39. As a developer, I want a trailing space after the pasted Invocation, so that I can keep typing arguments.
40. As a developer, I want the paste to go into whatever app has focus on macOS and Windows, so that it works in any terminal or editor.
41. As a developer, I want my existing clipboard restored after a paste, so that I don't lose what I copied.
42. As a developer, I want the Pad to hide itself before pasting if it's visible, so that text lands in my app and not in the Pad.
43. As a developer, I want pastes from multiple phones to run one at a time, so that they never interleave.
44. As a macOS user, I want a guided Accessibility permission check, so that I know why paste isn't working and how to fix it.
45. As a macOS user, I want sharing disabled until Accessibility access is granted, so that taps never silently fail.
46. As a developer, I want multiple phones paired at once, so that I can use an iPhone and a Samsung together.
47. As a developer, I want the Pad to list connected phones with name, online dot and last seen, so that I know who is connected.
48. As a developer, I want phones auto-named from their browser and renamable, so that I can tell them apart.
49. As a developer, I want a phone remembered across sharing sessions by a device ID, so that it keeps its View.
50. As a developer, I want to drag a Harness chip onto a phone's card, so that the phone shows that Harness's skills.
51. As a developer, I want to drag a Skill Directory chip onto a phone's card, so that I can give it just one folder.
52. As a developer, I want to remove a chip from a phone with an ×, so that I can narrow its View.
53. As a developer, I want View changes to apply live on the phone, so that I see results immediately.
54. As a developer, I want newly scanned phones to see everything by default, so that setup is frictionless.
55. As a phone user with nothing assigned, I want a clear empty message, so that I know to ask at the computer.
56. As a developer, I want to Disconnect a phone, so that it's revoked until the next scan.
57. As a developer, I want to Forget a phone, so that its saved View is deleted.
58. As a developer, I want to save the arrangement of all phones as a named Profile, so that I can recreate it later.
59. As a developer, I want to switch, rename and delete Profiles from a dropdown, so that I can change setups in one step.
60. As a developer, I want a Profile entry for an unknown phone to be skipped safely, so that stale Profiles don't break.
61. As a developer, I want my directories, hotkey, templates, phones and Profiles saved in the OS config folder, so that they persist across restarts.
62. As a developer, I want an optional start-at-login toggle, so that the app is ready when I am.
63. As a developer, I want Skills Pad to offer to install hooks for Claude Code and Codex, so that Sessions can report their Status.
64. As a developer, I want to see exactly what will change in my config before consenting, so that nothing is modified silently.
65. As a developer, I want one-click removal of the installed hooks, so that I can undo it cleanly.
66. As a developer, I want a card per Session with project folder, Status and elapsed time, so that I can see all my agents at a glance.
67. As a developer, I want Status to distinguish working, needs input, idle, stuck and ended, so that I know which agent needs me.
68. As a developer, I want a Session marked stuck when it is working with no events for 5 minutes, so that stalls surface.
69. As a developer, I want the stuck threshold to be configurable, so that long builds don't cry wolf.
70. As a developer, I want a phone's View to filter Sessions by Harness as well as skills, so that each phone shows a coherent slice.
71. As a developer, I want Status shown in the Pad as well as on the phone, so that I can check at the desk.
72. As a developer, I want sessions that started before hooks were installed to appear once they restart, so that I understand why one is missing.
73. As a developer, I want to tap any Session card and choose "Pending tickets", so that I can see what work is left in that agent's project.
74. As a developer, I want each pending Ticket shown with its title, status and checklist progress, so that I can judge what to hand the agent next.
75. As a developer, I want the ticket folder pattern to be configurable, so that projects with a different layout still work.
76. As a developer, I want each Ticket to show its status as a tag, such as triage or ready-for-agent, so that I can see at a glance which ones are ready to build.
77. As a developer, I want an "Implement" button on a pending Ticket, so that one tap tells the agent to start building it.

## Implementation Decisions

- **Delivery:** a single Rust binary per OS (macOS app and Windows executable). The phone page is embedded in the binary and served by the app.
- **Core vs shell:** a headless **Core** (a Rust library crate) holds all behavior. The shell is a Tauri app: the Pad window is HTML/CSS/JS in the system web view (WebKit on macOS, WebView2 on Windows), built from the approved prototype in `prototypes/skills-pad.prototype.html`. The shell, tray icon and hotkey registration call Core commands and hold no logic.
- **Crates in scope:** Tauri 2 for the Pad window, with its tray support and the official global-shortcut and autostart plugins; a small HTTP server crate (`tiny_http`) for the phone and hook endpoints; `qrcode` for the QR; `arboard` for the clipboard; `enigo` for simulated paste. Chosen over egui (less polished UI, no reuse of the HTML prototype) and Electron (much larger and slower to start for an always-running tray app). The Rust toolchain must be installed first (`rustup`).
- **Skill scanning:** a registry of known Harnesses (Claude Code, Codex, Cursor, Gemini CLI, OpenCode, Copilot), each with global skill locations and an Invocation template. A Harness yields a chip only if skills are found. The generic `~/.agents` folder is its own "Shared" chip. User-added Skill Directories are assigned to an existing or custom Harness. Skills are rescanned on every list request and are never cached. Plugin versions are de-duplicated, keeping the newest.
- **Invocation templates:** verified from each Harness's docs: Claude `/name` (plugins `/plugin:skill`), Codex `$name`, Cursor `/name`, Copilot `/name`, OpenCode `@name`. Gemini CLI has no typed form, so it and the Shared chip use the plain-text default (`Use the <name> skill `). Every template is user-editable, and every Invocation ends in a trailing space.
- **Paste flow:** the phone sends a skill ID only, never arbitrary text. The Core builds the Invocation, hides the Pad if visible, waits about 150 ms for focus to return, saves the clipboard, sets the text, simulates Cmd+V (macOS) or Ctrl+V (Windows), then restores the clipboard. Pastes are serialized through one queue.
- **Sharing lifecycle:** the server runs only while sharing. Starting generates a new token and a random port on the chosen LAN address. The token travels in the QR URL and then lives in a cookie. Phone page heartbeats keep the session alive; 15 minutes without activity or "Stop sharing" shuts the server down. Traffic is plain HTTP by decision. A Phone has a long-lived device ID cookie so it is recognized across sessions.
- **Phones, Views and Profiles:** the Pad holds a list of known Phones (name, device ID, last seen, View). A View is a set of Harness chips and Skill Directory chips; dropping a Harness chip includes all its directories. New Phones default to everything. A Profile is a named snapshot of all Phone Views, matched by device ID, with unknown IDs skipped. Disconnect revokes until the next scan; Forget deletes saved state.
- **Wake lock:** the phone page uses the Screen Wake Lock API where available and a hidden-looping-video fallback otherwise (the API requires a secure context, which plain LAN HTTP isn't). It releases when the page is hidden or the phone locks, and re-acquires on return.
- **Platform behavior:** macOS requires Accessibility permission; the Pad detects its absence, guides the user to System Settings and blocks sharing until granted. Windows shows a firewall prompt the first time and the Pad explains it. Start-at-login is an optional toggle, off by default.
- **Config:** a JSON file in the OS config directory stores added directories, custom Harnesses, templates, hotkey, network override, Phones, Profiles and the stuck threshold.
- **Session status (slice 5):** the app installs a small hook command that forwards Claude Code and Codex lifecycle events (session start, prompt submitted, before and after tool use, permission requested or notification, stop, interrupt, session end) to a local-only listener. Status is derived: working after a prompt or tool event until stop; needs input on a permission event; idle after stop; ended on session end; stuck when working with no events for the threshold (default 5 minutes). There is no true percentage progress; the "bar" is a state indicator with elapsed time and tool-call count. Installing hooks edits the user's Claude and Codex config, so it requires explicit consent, shows the change first, and supports one-click removal. Cursor, Gemini CLI, OpenCode and Copilot are not covered until their hook support is verified.
- **Pending tickets:** a Session card offers "Pending tickets" for its project folder. Tickets are Markdown files matched by a configurable pattern (default `.scratch/*/issues/*.md`). They are read on every request and never cached. A Ticket is pending unless its `**Status:**` is done, closed or wontfix. The phone sends a Session ID only, and the Core resolves the folder, so arbitrary paths are never read.
- **Implement command:** a pending Ticket has an "Implement" button that pastes a per-Harness implement template (default `Implement the ticket at <path> `) and presses Enter, through the normal paste flow. The phone sends only a Session ID and a Ticket ID.
- **Build order:** (1) scan and list skills; (2) phone page and paste on macOS; (3) Windows paste; (4) multi-phone Views and Profiles; (5) Session status.

## Testing Decisions

- A good test checks external behavior only: what a phone or hook sender can observe over the Core's interfaces, or what a Pad action changes. Tests never reach into internal state.
- **Primary seam:** a headless Core started on a random port. Phone and hook behavior is tested over real HTTP. Pad actions (start and stop sharing, assign or remove a chip, save or switch a Profile, add a Skill Directory, rename, disconnect, forget) are tested as direct Core commands.
- **Faked edges:** (1) a temp home directory and temp Skill Directories containing fixture skills in the Claude plugin-cache, Codex, shared and added-folder layouts; (2) a recording Paster that stands in for the clipboard and simulated keystrokes, so tests assert the exact text, order and clipboard restore; (3) an injectable clock for the 15-minute idle stop and the stuck threshold.
- **Behaviors to cover:** the skill list per Harness and per View; de-duplication of plugin versions; Invocation text per Harness and after template edits; paste serialization across two phones; token and device-ID rules (old token rejected, disconnected phone rejected, unknown skill ID rejected); idle stop and heartbeat; Profile save and switch, including unknown phones; Status transitions from hook events including stuck; View filtering of Sessions.
- **Manual checks per slice:** the real paste on macOS and on Windows, Accessibility guidance, the status-bar icon, the global hotkey, drag and drop in the Pad, wake lock on real iOS and Android phones, and the firewall prompt.
- **Prior art:** none, because the repository is empty. The seams above are the new convention.

## Out of Scope

- Hiding individual skills per phone, favorites and recents.
- HTTPS or certificates for the phone page.
- A native iOS or Android app.
- Per-phone approval of the first pairing.
- A true percentage progress bar for agents.
- Hook-based Session status for Cursor, Gemini CLI, OpenCode and Copilot until their hook support is verified.
- Pasting the full SKILL.md body instead of the Invocation.

## Further Notes

- The shared vocabulary is in `GLOSSARY.md`: Skill, Harness, Skill Directory, Invocation, Paste, Phone, Pairing, View, Profile, Session, Status and Pad.
- No ADRs were written; none of the decisions were both hard to reverse and surprising.
- Open facts to verify before or during the build: the hook event names each Harness currently supports. (Invocation forms were verified in ticket 02.)
