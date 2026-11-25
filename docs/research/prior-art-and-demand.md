# Prior art and demand for Skills Pad

Checked on 2026-11-25. GitHub star counts and last-push dates come from the GitHub API (`gh api repos/<owner>/<repo>`) on that date. "Verified" means read on the primary source (official docs, the repo README, the actual issue thread). "Third-party" means only a secondary source was found.

Skills Pad's three capabilities, used as columns below:

- **Pick**: browse/search installed skills across harnesses.
- **Paste**: a phone tap inserts the invocation (`/name`, `$name`) at the cursor of the focused desktop app.
- **Status**: live per-session status (working / needs input / idle / stuck / ended).
- **Jump**: tap a session to bring its terminal tab/window to the front on the computer.

## 1. Bottom line

Nothing found does all of Skills Pad's jobs together: list skills across harnesses, paste one with a phone tap, show cross-harness session status, and jump to the session's terminal. Each piece exists on its own, often done better. The closest matches are (a) **JulienCr/streamdeck-claude** and **paultyng/agentsd**, Stream Deck plugins that show live Claude Code session state on keys. A key press focuses the session's Warp tab or Ghostty window. Both need a Stream Deck and only cover Claude Code. (b) **Anthropic's Remote Control and Codex in the ChatGPT app** are official phone apps that already send "needs input" pushes and let you answer from the phone. (c) **Skills Manager (xingkongliang)** and **`npx skills`** list and sync skills across 50+ agents. Demand for the status/notification part is well documented: the Claude Code "waiting for input" hook requests have 81 and 48 thumbs-up, and the Codex "Remote Control" request has 411. Both vendors have since shipped native answers, which weakens that part of the pitch. Demand for the skill-picking part is weaker but real: people ask for fuzzy matching and complain about long `/` menus, and one closed request asked for one-tap slash-command buttons on mobile. Skills Pad's most distinct parts are paste-into-whatever-is-focused (no harness integration) and one cross-harness status view that also jumps to the right tab. If the user is happy inside the official mobile apps, the status board is largely redundant. The cross-harness, no-account, LAN-only, "phone as a skill keypad" angle has no direct competitor that I found.

## 2. Comparison table

Legend: Y = yes, P = partial, N = no, ? = not verified.

| Product | Pick | Paste | Status | Jump | Platform | Open source? | Activity (2026-11-25) | Link |
|---|---|---|---|---|---|---|---|---|
| Claude Code Remote Control + Claude app | N (can type `/` commands, no skill browser verified) | N (sends messages into the session, no paste into an arbitrary app) | Y (push when actions are required, session list with online dot) | N | Claude iOS/Android/web; host is the CLI, Desktop or VS Code | No (first-party, needs Pro/Max/Team/Ent) | Active, official docs | [docs](https://code.claude.com/docs/en/remote-control) |
| Claude Code agent view (`claude agents`) | N | N | Y (Needs input / Working / Idle / Completed…) for background sessions only | N | Terminal | No | Official | [docs](https://code.claude.com/docs/en/agent-view) |
| Codex in ChatGPT mobile app | N | N | Y (threads, approvals, answer questions) | N | ChatGPT iOS/Android; host is the Codex **app** on macOS (Windows "coming soon" as of 2026-05-14) | No | Launched 2026-05-14 | [announcement](https://openai.com/index/work-with-codex-from-anywhere/) |
| Cursor web & mobile (cloud agents) | N | N | P (cloud agents only) | N | iOS app (beta) + PWA | No | Official | [docs](https://cursor.com/docs/cloud-agent/web-and-mobile) |
| GitHub Copilot cloud agent on GitHub Mobile | N | N | P (cloud agent tasks/PRs) | N | GitHub Mobile | No | Official | [docs](https://docs.github.com/en/copilot/how-tos/use-copilot-agents/cloud-agent/use-cloud-agent-on-mobile) |
| JulienCr/streamdeck-claude | N | N | Y (Claude Code) | Y (Warp tab; VS Code window) | Stream Deck on macOS/Windows/WSL | MIT | 7 stars, pushed 2026-11-25 | [repo](https://github.com/JulienCr/streamdeck-claude) |
| paultyng/agentsd | N | N | Y (Claude Code, plus approve/deny) | P (Focus button brings Ghostty/Claude Desktop forward) | Stream Deck, macOS 13+ | MIT | 11 stars, pushed 2026-08-12 | [repo](https://github.com/paultyng/agentsd) |
| Elgato "Claude Code Shortcut Profile" | P (57 fixed slash commands) | ? (how it types not stated) | N | N | Stream Deck | Marketplace profile | Article updated 2026-08-07 | [Elgato](https://www.elgato.com/explorer/products/stream-deck/control-claude-with-stream-deck-for-faster-workflows/) |
| CC Pocket (K9i-0/ccpocket) | P (slash autocomplete, third-party claim) | N (chats with session via bridge) | Y (approvals, questions) | N | iOS/Android app + bridge on mac/Linux/Win; QR pairing | MIT | 1,112 stars, release 2026-11-25 | [repo](https://github.com/K9i-0/ccpocket) |
| CloudCLI / claudecodeui | ? | N | P | N | Web UI + desktop companion | AGPL-3.0 | 14,759 stars, release v1.37.10 on 2026-11-25 | [repo](https://github.com/siteboon/claudecodeui) |
| MobileCLI, Pockode, PocketDex, codex-remote-control-lab, Loopsy | N | N (they stream or drive the terminal) | P/Y | N | Phone + local daemon, often QR/LAN | Various | 16–139 stars | see §3 |
| Claude Status Bar (m1ckc3s) | N | N | Y (Claude Code, multi-session) | P (exact per-tab focus as opt-in beta, issue #19) | macOS menu bar | MIT | 741 stars, v0.4.5 on 2026-09-18 | [repo](https://github.com/m1ckc3s/claude-status-bar) |
| ClaudeDot, Pulse, AgentBar, AgentPing… | N | N | Y (Claude Code; AgentBar also Copilot CLI) | N/? | macOS menu bar | Mostly MIT | 0–13 stars | see §3 |
| claude-squad | N | N | P (tmux list of agents) | Y (inside its own TUI) | Terminal (tmux) | AGPL-3.0 | 8,867 stars, pushed 2026-11-13 | [repo](https://github.com/smtg-ai/claude-squad) |
| Crystal → Nimbalyst, Conductor | N | N | P | Y (own app) | Desktop (Conductor: Mac) | Crystal MIT; Conductor closed | Crystal 3,128 stars, pushed 2026-02-26 | [Crystal](https://github.com/stravu/crystal), [Conductor](https://www.conductor.build/) |
| Skills Manager (xingkongliang) | Y (50+ agents) | N | N | N | Desktop macOS/Windows (+?) | MIT | 7,776 stars, v1.44.0 on 2026-11-12 | [repo](https://github.com/xingkongliang/skills-manager) |
| `npx skills` (vercel-labs/skills) | Y (CLI install/manage) | N | N | N | CLI | MIT | 40,859 stars, pushed 2026-11-17 | [repo](https://github.com/vercel-labs/skills) |
| Skiller, Anvil, Skilldeck, Chops, Skill Cabinet | Y | N | N | N | Desktop (several macOS-only) | Mostly MIT | 0–56 stars | see §3 |
| Raycast: Skills (keito4), Agent Executor, Claude Code Cheatsheet | P | N (no extension that inserts into a session was found) | N | N | macOS (Raycast) | Raycast store | — | [Skills](https://www.raycast.com/keito4/skills) |
| Touch Portal | N (manual config) | P (key-press simulation; arbitrary-text typing not confirmed on the page) | N | N | Android/iOS → Win/mac/Linux | Paid Pro $13.99 | Active | [site](https://www.touch-portal.com/) |
| Macro Deck | N (manual config) | ? | N | N | Phone browser → Windows | Apache-2.0 | 1,913 stars, pushed 2026-11-24 | [repo](https://github.com/Macro-Deck-App/Macro-Deck) |

## 3. Notable competitors in detail

### Official vendor features (the biggest overlap)

- **Claude Code Remote Control** (verified, [docs](https://code.claude.com/docs/en/remote-control)). This feature "connects claude.ai/code or the Claude app for iOS and Android to a Claude Code session running on your machine." You start it with `claude remote-control` (server mode, up to 32 concurrent sessions by default), `claude --remote-control`, or `/rc`. The CLI shows a QR code. Sessions appear in the app's Code list "with a green status dot when online." **Mobile push notifications** can be enabled in `/config`, with "Push when actions required" for permission prompts and questions. Limits: it needs a Pro/Max/Ultra/Team/Enterprise plan ("API keys are not supported"), traffic goes through Anthropic's servers, and some commands are local-only. The docs list which commands work from mobile. Whether arbitrary user skills can be invoked from the phone is not stated explicitly. The skills doc says skill visibility settings affect "the command lists advertised to Remote Control clients" ([skills docs](https://code.claude.com/docs/en/skills)), which implies they can be.
- **Claude Code agent view** (verified, [docs](https://code.claude.com/docs/en/agent-view)): "one screen for all your background sessions: what's running, what needs your input, and what's done". However, "Interactive sessions you have open in other terminals don't appear until you background them."
- **OSC 7501 status in the terminal** (verified, [terminal-config](https://code.claude.com/docs/en/terminal-config)): Claude Code v2.1.295+ reports working / waiting / done to terminals that implement the protocol. Otherwise it puts a status glyph in the tab title.
- **Codex in the ChatGPT mobile app** (verified, [OpenAI, 2026-05-14](https://openai.com/index/work-with-codex-from-anywhere/)). You can work across threads, approve commands, answer clarifying questions, and start new threads. It pairs with the **Codex app on macOS** through a relay. Windows support was "coming soon." The announcement does not say that terminal-only Codex CLI sessions are supported. Third-party summaries of the remote-connections docs say setup "starts from the Codex App; you cannot set it up from the Codex CLI or IDE Extension". I could not fetch [learn.chatgpt.com/docs/remote](https://learn.chatgpt.com/docs/remote) (404 when fetched), so that claim is **third-party only**.
- **Cursor** ([docs](https://cursor.com/docs/cloud-agent/web-and-mobile)) and **GitHub Copilot** ([docs](https://docs.github.com/en/copilot/how-tos/use-copilot-agents/cloud-agent/use-cloud-agent-on-mobile)): their mobile surfaces drive **cloud** agents only. Neither touches local sessions or local skills.

### Stream Deck plugins (closest match for status + jump)

- **JulienCr/streamdeck-claude** (verified README): "Each running `claude` CLI session lights up one key on your deck — project name, current state… pulsing when it needs you. Press a key to copy the session's `cwd` to your clipboard and, when possible, bring its terminal to the foreground." It focuses Warp tabs, and raises VS Code windows at window level only. It runs on macOS 12+ and Windows 10+, needs a Stream Deck, and covers Claude Code only.
- **paultyng/agentsd** (verified README): it shows status states (`Working`, `Permission?`, `Question?`, `Idle`, `Error`), lets you approve or deny from the deck, and has a Focus button that brings Ghostty or Claude Desktop forward. It uses 16 Claude Code hook events. It runs on macOS 13+ only.
- **Elgato Claude Code Shortcut Profile**: "57 slash commands across 8 folders" (Elgato page). It is the closest thing to a "skill keypad," but it is a fixed list and does not discover your installed skills.

### Phone remotes for agents (community)

- **CC Pocket** ([repo](https://github.com/K9i-0/ccpocket)): a native app for Codex, Claude and Antigravity. It uses a bridge server, connects by QR code or Tailscale, and offers chat, approvals and questions. It is the most active community project here (1,112 stars, release today).
- **CloudCLI / claudecodeui** ([repo](https://github.com/siteboon/claudecodeui)): a web UI for Claude Code, Cursor CLI, Codex and OpenCode, with a paid hosted option.
- **MobileCLI** (95 stars), **Pockode** (57), **PocketDex** (16, QR pairing for Codex CLI), **codex-remote-control-lab** (19, "token-protected LAN phone bridge"), **Loopsy** (139). All of these stream or drive the agent itself. None pastes into the focused desktop app.

### Status monitors (Claude Code hooks)

- **Claude Status Bar** (741 stars): a macOS menu bar app that shows the highest-priority session. A session "awaiting your permission is never hidden behind one that's thinking". Per-tab focus was tracked in [issue #19](https://github.com/m1ckc3s/claude-status-bar/issues/19), now closed as an opt-in beta.
- Smaller ones: [ClaudeDot](https://github.com/poorfish/ClaudeDot), [Pulse](https://github.com/sayantan94/Pulse), [AgentBar](https://github.com/jreed91/agentbar) (Claude Code and Copilot CLI), [AgentPing](https://ericermerimen.github.io/agentping) (not checked on GitHub). All are macOS only. None has a phone view.

### Skill managers

- **Skills Manager** ([repo](https://github.com/xingkongliang/skills-manager)): "manage, sync, and organize AI agent skills across 50+ coding tools." It has a per-agent "Global Workspace" that lists every skill in that agent's folder, plus a marketplace, presets and sync. It does not invoke or paste skills.
- **`npx skills`** ([vercel-labs/skills](https://github.com/vercel-labs/skills), 40.9k stars): "The open agent skills tool." A CLI for installing and managing skills.
- Others: [Skiller](https://github.com/beautyfree/skiller), [Anvil](https://github.com/s0up4200/anvil), [Skilldeck](https://github.com/aliaihub/skilldeck), [Chops](https://github.com/hnshah/chops), [Skill Cabinet](https://flaviocopes.com/skill-cabinet/) (blog post by its author).

### Generic phone macro pads

- **Touch Portal** ([site](https://www.touch-portal.com/)): Android/iOS to Windows/macOS/Linux, with key-press simulation. Free with 2 pages, Pro is $13.99. You could hand-build a skill pad with it, but it has no skill discovery or agent status.
- **Macro Deck** ([repo](https://github.com/Macro-Deck-App/Macro-Deck), Apache-2.0, 1.9k stars): the same idea, Windows host.
- **Stream Deck Mobile** and **Unified Remote**: not checked in this pass.

## 4. Demand evidence

### Status / "which agent needs me" (strong, but vendors have responded)

- anthropics/claude-code [#13024](https://github.com/anthropics/claude-code/issues/13024), "Add hook for when Claude is waiting for user input": 81 thumbs-up, 27 comments, closed as completed 2026-08-17.
- anthropics/claude-code [#10168](https://github.com/anthropics/claude-code/issues/10168), "Add hook for user input/question events": 48 thumbs-up, closed as completed 2026-08-17.
- anthropics/claude-code [#36885](https://github.com/anthropics/claude-code/issues/36885): "When running multiple Claude Code sessions simultaneously… there's no way to know which session is blocked waiting for user input." Closed as not planned (stale).
- openai/codex [#3962](https://github.com/openai/codex/issues/3962), "Play a sound when Codex finishes a prompt / task": 203 thumbs-up, still open.
- openai/codex [#3247](https://github.com/openai/codex/issues/3247), "Support notify for user approval events": 34 thumbs-up.
- openai/codex [#2109](https://github.com/openai/codex/issues/2109), "Event Hooks": 525 thumbs-up, closed. Hooks have since shipped.
- Product Hunt discussions (third-party, not individually verified): ["Your AI agent is waiting for you. Your attention isn't."](https://www.producthunt.com/p/vibecoding/your-ai-agent-is-waiting-for-you-your-attention-isn-t-how-much-time-do-you-lose-to-this) and ["Running 3-4 Claude Code / Codex sessions at once: what breaks first for you?"](https://www.producthunt.com/p/vibecoding/running-3-4-claude-code-codex-sessions-at-once-what-breaks-first-for-you).
- Supply-side signal: there are at least 6 hook-based menu bar monitors and 2 Stream Deck status plugins, and Claude Status Bar alone has 741 stars.

### Remote / mobile control (strong, now served officially)

- openai/codex [#9224](https://github.com/openai/codex/issues/9224), "Codex Remote Control": "The ability to remotely control `codex` cli running on my desktop PC from my phone." 411 thumbs-up, closed. Codex mobile shipped 2026-05-14.
- openai/codex [#23200](https://github.com/openai/codex/issues/23200) asks for headless Linux hosts for Codex mobile "without requiring the desktop app to stay online" (68 thumbs-up, open). This shows a gap for CLI users.
- CC Pocket has 1.1k stars and CloudCLI has 14.8k stars.

### Skill discovery / picking (moderate)

- anthropics/claude-code [#58787](https://github.com/anthropics/claude-code/issues/58787): "With many plugins/skills installed… the `/` menu becomes very long and hard to scan. Finding a built-in command means scrolling past dozens of skills." Closed as duplicate.
- anthropics/claude-code [#24355](https://github.com/anthropics/claude-code/issues/24355), fuzzy/keyword matching for slash commands. The workaround it lists is to "memorize exact skill names." Closed as completed.
- anthropics/claude-code [#60167](https://github.com/anthropics/claude-code/issues/60167), "[Mobile] Floating button shortcut for slash commands": "invoking a slash command requires… Type the slash command character-by-character (no autocomplete on mobile keyboard)." 0 thumbs-up, closed as not planned on 2026-06-16. This is the closest stated need to Skills Pad's picker, but it has no traction.
- Supply-side signal: Skills Manager (7.8k stars) and `npx skills` (40.9k) show that managing many skills is a real pain. Neither of them invokes skills.

### Counter-evidence (reasons Skills Pad may be unnecessary)

- Claude Code already sends mobile pushes when actions are required, and the official app lists sessions with an online dot ([docs](https://code.claude.com/docs/en/remote-control#mobile-push-notifications)). It also has a built-in desktop notification path, OSC 7501 tab status, and agent view.
- Codex mobile covers approvals and questions for Codex app users.
- The in-harness `/` and `$` pickers already autocomplete, and fuzzy matching was requested and closed as completed (#24355). Typing `/td<Tab>` on the keyboard may beat reaching for a phone.
- The "jump to tab" idea is already done for some terminals (Warp, Ghostty) by the Stream Deck plugins, and per-tab focus for Claude Status Bar has an opt-in beta.

## 5. Feasibility facts (verified on primary docs)

### Claude Code hooks ([docs](https://code.claude.com/docs/en/hooks))

- The relevant events exist: `SessionStart`, `UserPromptSubmit`, `PreToolUse`, `PermissionRequest`, `PostToolUse`, `Notification`, `Stop`, `StopFailure`, `SubagentStart/Stop`, `SessionEnd`, and more.
- `Notification` matches on `notification_type`, including `permission_prompt`, `idle_prompt`, `elicitation_dialog`, `agent_needs_input` and `agent_completed`. Together these cover "needs input."
- Every event receives `session_id`, `transcript_path`, `cwd`, `permission_mode` and `hook_event_name` as JSON.
- Hook types include `command` (stdin) and **`http`** ("sends the input as a POST request to a URL"). Skills Pad could receive events on a localhost HTTP endpoint without a shim script, as agentsd does on port 9200.
- Skill invocation: "invoke one directly with `/skill-name`". Plugin skills are namespaced "as `/plugin-name:skill-name`". The locations are `~/.claude/skills/<name>/SKILL.md`, project `.claude/skills/`, enterprise managed dir, and `<plugin>/skills/` ([skills docs](https://code.claude.com/docs/en/skills)). This matches the spec.

### Codex hooks ([docs](https://learn.chatgpt.com/docs/hooks); developers.openai.com/codex/hooks redirects there)

- Events (14): `PreToolUse`, `PermissionRequest`, `PostToolUse`, `PreCompact`, `PostCompact`, `UserPromptSubmit`, `SubagentStop`, `Stop`, `PreModelRequest`, `PostModelRequest`, `Interrupt`, `SessionStart`, `SubagentStart`, `SessionEnd`.
- Input includes `session_id`, `transcript_path`, `cwd`, `hook_event_name` and `model`, plus `turn_id` for turn-scoped events.
- "Hooks are enabled by default". They can be disabled with `[features] hooks = false`. Non-managed hooks "must be reviewed and trusted before they run". That first-run trust step affects Skills Pad's install flow.
- Config: `~/.codex/hooks.json` (`%USERPROFILE%\.codex\hooks.json` on Windows), or `[hooks]` in `config.toml`. There is a `commandWindows` override.
- **`PermissionRequest`** "runs when Codex is about to ask for approval". **No separate "needs input / question" hook is documented.** A Codex clarifying question will probably look like `Stop` (turn ended). So "needs input" vs "idle" can only be told apart for approvals.
- `prompt`/`agent` handler types "are parsed but skipped". Only command handlers run, so an HTTP POST needs a small command shim (e.g. `curl`).
- Legacy `notify` in `config.toml` exists. The [sample config](https://developers.openai.com/codex/config-sample) shows `notify = ["notify-send", "Codex"]` commented out. Per third-party posts, it fires only on `agent-turn-complete` ([example](https://codex.danielvaughan.com/2026/04/10/codex-cli-agent-notifications-desktop-alerts-monitoring/)). That detail was not verified on the official reference.

### Codex skill invocation ([docs](https://learn.chatgpt.com/docs/build-skills); developers.openai.com/codex/skills redirects there)

- "In CLI/IDE, run `/skills` or type `$` to mention a skill", and the docs give the example `$skill-creator`. **`$name` is confirmed.**
- Locations: `$CWD/.agents/skills`, parent dirs up to `$REPO_ROOT/.agents/skills`, user `$HOME/.agents/skills`, admin `/etc/codex/skills`, plus bundled system skills. The page does **not** list `~/.codex/skills`. The spec should treat `~/.agents/skills` as Codex's user location. Per Skill Cabinet's author, several agents share it, which matches the spec's "Shared" chip.

### Native remote features that overlap

- Claude Code: Remote Control, mobile push, agent view and OSC 7501 (see §3). Remote Control needs a claude.ai subscription and works through Anthropic's servers. It does not work with API keys or when `ANTHROPIC_BASE_URL` points to a gateway ([requirements](https://code.claude.com/docs/en/remote-control)). Skills Pad's LAN-only, account-free design is still different for those users.
- Codex: phone control through the ChatGPT app, tied to the Codex desktop app (macOS at launch).

## 6. Open questions / not verified

- Whether a user-installed skill (`/my-skill`) can be invoked from the Claude mobile app over Remote Control. The docs imply skills are advertised to Remote Control clients but do not say so explicitly. Open issues [#68252](https://github.com/anthropics/claude-code/issues/68252) and [#63924](https://github.com/anthropics/claude-code/issues/63924) report some slash commands not executing over Remote Control.
- Whether Codex mobile can drive a **terminal** Codex CLI session, or only the Codex app. The official remote docs page returned 404 to my fetcher, and the "app only" claim comes from secondary sources.
- Current Windows status of Codex phone pairing (it was "coming soon" on 2026-05-14). Open issue [#28919](https://github.com/openai/codex/issues/28919) mentions a missing Windows "control other devices" tab.
- Whether the Elgato Claude Code Shortcut Profile types into the terminal, and its price.
- CC Pocket's "slash-command autocomplete" comes from a third-party listing and is not in the README lines I checked.
- Stream Deck Mobile, Unified Remote, Conductor's status/notification features and pricing, and Hacker News threads on this topic were not found or not checked. I found no HN thread through search.
- Reddit claims, such as "more than ~15 skills breaks things", were only seen through aggregators and are not verified.
- Gemini CLI, OpenCode and Copilot CLI hook/skill syntax was out of scope for this pass. Copilot CLI hooks are used by AgentBar, per its search snippet only.
