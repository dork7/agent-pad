# Skills Pad

A desktop app that lists the agent skills installed on your computer and lets you pick one from your phone, which then pastes its invocation at the cursor of whatever app has focus on the computer.

## Language

**Skill**:
A named, reusable instruction package for a coding agent, defined by a `SKILL.md` file.
_Avoid_: Command, prompt, plugin

**Harness**:
A coding agent product that skills belong to and are invoked in, such as Claude Code or Codex. Discovered by scanning, not fixed; the user can define custom ones.
_Avoid_: Tool, agent, client

**Skill Directory**:
A folder that Skills Pad scans for skills. Installed global locations are included by default; the user can add more.
_Avoid_: Source, library, repo

**Invocation**:
The harness-specific text that triggers a skill when typed into that harness, such as `/graphify` for Claude or `$graphify` for Codex.
_Avoid_: Command, snippet

**Paste**:
Inserting a skill's invocation at the cursor of the focused app on the computer.
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
A session's current state: working, needs input, idle, stuck, or ended. Stuck means working with no events for a set time.
_Avoid_: Progress, health

**Pad**:
The desktop window and status-bar icon of Skills Pad, summoned by a hotkey, used for pairing and settings.
_Avoid_: Panel, dashboard
