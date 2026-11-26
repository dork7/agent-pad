# 03: On-demand sharing with QR, token and idle stop

**What to build:** A minimal **Console** window with "Open on phone" starts the LAN server on the chosen address and a random port, mints a fresh token and shows a QR code plus the address as text. Requests without the token are rejected. Sharing stops on "Stop sharing" or after 15 minutes without phone activity.

**Blocked by:** #2

**Status:** done

**GitHub:** #4

- [x] Nothing listens on the network until sharing is started
- [x] Each start produces a new token; a token from a previous start is rejected
- [x] The primary LAN address is picked automatically and can be overridden in the Console
- [x] Heartbeats from an open phone page keep sharing alive; 15 minutes of silence stops it
- [x] "Stop sharing" stops the server immediately

Ticket 03 delivers the Core commands and Phone endpoints. The "Open on phone"
button and other Console controls land with ticket 05. The phone page lands with
ticket 04.
