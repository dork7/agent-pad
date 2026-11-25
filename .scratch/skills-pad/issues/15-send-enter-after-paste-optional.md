# 15: Send Enter after paste (optional)

**What to build:** An optional toggle in the **Pad**, off by default, that makes every **Paste** finish with Enter so a tap runs the **Skill** instead of only inserting its **Invocation**.

**Blocked by:** #5

**Status:** ready-for-agent

**GitHub:** #16

- [ ] With the toggle off, behavior is unchanged: only the Invocation and trailing space are pasted
- [ ] With it on, Enter is sent after the paste on macOS and Windows
- [ ] The toggle persists across restarts and applies to the next tap without restarting
- [ ] Serialized pastes from several phones each get their own Enter, in order (tested with the recording Paster)
