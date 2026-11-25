# 03: On-demand sharing with QR, token and idle stop

**What to build:** A minimal **Pad** window with "Open on phone" starts the LAN server on the chosen address and a random port, mints a fresh token and shows a QR code plus the address as text. Requests without the token are rejected. Sharing stops on "Stop sharing" or after 15 minutes without phone activity.

**Blocked by:** #2

**Status:** ready-for-agent

**GitHub:** #4

- [ ] Nothing listens on the network until sharing is started
- [ ] Each start produces a new token; a token from a previous start is rejected
- [ ] The primary LAN address is picked automatically and can be overridden in the Pad
- [ ] Heartbeats from an open phone page keep sharing alive; 15 minutes of silence stops it
- [ ] "Stop sharing" stops the server immediately
