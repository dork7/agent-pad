# Agent Pad

A desktop app that shows your running coding agents and their status at a glance, and lets you pick a skill from your phone to paste at the cursor of whatever app has focus on the computer.

## Language

**Skill**:
A named, reusable instruction package for a coding agent, defined by a `SKILL.md` file.
_Avoid_: Command, prompt, plugin

**Harness**:
A coding agent product that skills belong to and are invoked in, such as Claude Code or Codex. Discovered by scanning, not fixed; the user can define custom ones.
_Avoid_: Tool, agent, client

**Skill Directory**:
A folder that Agent Pad scans for skills. Installed global locations are included by default; the user can add more.
_Avoid_: Source, library, repo

**Invocation**:
The harness-specific text that triggers a skill when typed into that harness, such as `/graphify` for Claude or `$graphify` for Codex.
_Avoid_: Command, snippet

**Paste**:
The clipboard-and-keystroke step that inserts text into a window on the computer; it is how Run delivers an invocation.
_Avoid_: Type, send, inject

**Phone**:
The device the user scans the QR code with and taps skills on. It is a remote picker, never the paste target.
_Avoid_: Mobile app, client, remote

**Pairing**:
The act of a phone connecting to the desktop app over the local network after scanning its QR code.
_Avoid_: Login, session

**View**:
The set of harnesses and skill directories a particular phone is allowed to see. Each paired phone has its own view, chosen on the desktop.
_Avoid_: Profile, filter, permissions

**Profile**:
A named snapshot of every phone's view, so the whole arrangement can be switched in one step.
_Avoid_: Preset, layout, scene

**Session**:
One running conversation of an agent in a harness, in a project folder, as reported by that harness's hooks.
_Avoid_: Agent, process, run

**Status**:
A session's current state, shown on the phone: Running, Blocked (waiting for the user), Done (turn finished, idle), or Stalled (running with no events for a set time). Ended sessions are greyed and leave the list 10 minutes after they end.
_Avoid_: Progress, health, state

**Home**:
The phone's main screen: the list of running sessions with their status, most urgent first, plus an entry to all skills.
_Avoid_: Dashboard, main view

**Session screen**:
What opens when a session is tapped on Home: the session's header and the skill picker for that session's harness.
_Avoid_: Detail page

**Ticket**:
A Markdown file in a project's ticket folder that describes one piece of work, with a status and a checklist. Pending unless its status is done, closed or wontfix.
_Avoid_: Issue, task, card

**Console**:
The desktop window of Agent Pad, summoned by a hotkey or the status-bar icon, used for pairing, phones, Views, Profiles and settings.
_Avoid_: Pad, panel, dashboard

**HUD**:
The small always-on-top panel docked to a screen edge that shows one logo per harness in a ring, with a hover panel of sessions and requests.
_Avoid_: Widget, overlay, notch

**Skill picker**:
The skill list on the phone, in tabs per harness, where each skill offers Copy and Run.
_Avoid_: Skill pad, skill keypad

**Request**:
A harness asking the user for permission or an answer, which makes its session Blocked.
_Avoid_: Prompt, approval, notification

**Copy**:
Putting a skill's invocation on the computer's clipboard, without pasting.
_Avoid_: Paste, share

**Run**:
Raising a session's window, pasting a skill's invocation and pressing Enter, so the agent starts the skill. It never targets any other window and is unavailable while the session is Blocked.
_Avoid_: Execute, send, paste at cursor
