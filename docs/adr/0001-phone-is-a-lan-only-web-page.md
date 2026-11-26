# The phone is a LAN-only web page, not a native app

Agent Pad's phone side is a web page served by the desktop app over the local network, opened by scanning a QR code. We chose this over a native iOS app with a push relay (the approach Agent HUD takes) because it needs no accounts, no app store and no server of our own, and it works on Android too. The cost is permanent: no lock-screen or Dynamic Island tiles, no push notifications to a locked phone, and no access away from home, so alerts only work while the page is open and awake.
