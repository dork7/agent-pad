# 22: Status names and animated harness icons

**What to build:** The phone and the Console show each **Session**'s **Status** as Running, Blocked, Done or Stalled (Ended sessions leave the list 10 minutes after they end), replacing the current Working, Needs input, Idle and Stuck labels. Each session card carries its **Harness** icon, which animates by status: Running spins a ring (Claude's rays also turn, Codex's cursor blinks), Blocked pulses amber with an exclamation badge, Done breathes gently with a green check, Stalled shows a slow dashed red ring and a flicker, and Ended is grey and still. Done also covers idle (turn finished, waiting for the next prompt).

**Blocked by:** #14, #15

**Status:** ready-for-agent

**GitHub:** #20

- [ ] Status labels read Running, Blocked, Done and Stalled everywhere a status appears, and the glossary and docs use the same words
- [ ] Blocked covers permission requests, Done covers a finished turn, Stalled keeps the existing no-events-for-the-threshold rule
- [ ] Each harness shows its own icon; real logos are used where licensing allows and a neutral stand-in glyph otherwise
- [ ] Each status has its own motion and badge as described, and nothing moves when the phone asks for reduced motion
- [ ] Ended sessions are greyed and removed 10 minutes after they end
- [ ] Existing status tests are updated to the new names and still pass
