# 01: Scaffold, Core test seam, and Claude skill list over HTTP

**What to build:** Install the Rust toolchain and set up the project. A headless Core starts on a random port, and its tests use a temp home directory, a recording Paster and a fake clock. Global Claude **Skills** (including plugin versions de-duplicated to the newest) can be listed through a request to the Core.

**Blocked by:** None (can start immediately)

**Status:** ready-for-agent

**GitHub:** #2

- [ ] Rust toolchain installed and the project builds and tests on macOS
- [ ] Tests can start a Core on a random port against a temp home directory with fixture skills
- [ ] A list-skills request returns Claude skills with name and one-line description
- [ ] Duplicate plugin versions of the same skill appear once, newest version only
- [ ] The recording Paster and fake clock exist as test fakes for later tickets
