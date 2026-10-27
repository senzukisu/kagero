---
description: "How Rich Presence is implemented, configured and kept private."
icon: plug
---

# Discord Rich Presence

Kagero can show in your Discord profile that you are working in Kagero. It talks to the
**Discord desktop app on the same computer** through Discord's local IPC pipe
(`\\.\pipe\discord-ipc-N` on Windows). There is no server, no login, no token.

## How it is built

- Library: [`discord-rich-presence`](https://crates.io/crates/discord-rich-presence)
  1.x (MIT). It implements Discord's documented desktop RPC handshake and
  `SET_ACTIVITY` over the IPC pipe — the mechanism Discord provides for desktop games and
  apps without the full Social SDK. No custom protocol code.
- Module: `src/integrations/discord_rpc.rs`.
  - `build_presence` turns the app state + privacy settings into the desired activity.
  - `PresenceClient` is a transport-independent state machine (connect, send changes
    only, at most one update per 4 s, retry with back-off 5 s → 2 min, clear on disable).
    Tests drive it with a fake transport.
  - `DiscordPresence` runs it on a background thread; the UI only sends the desired
    state when it changes. Shutting Kagero down clears the activity and closes the pipe.
- If Discord isn't running, the settings page shows *Not connected* with the reason
  and retries in the background; no error dialogs appear.

## Application ID

Rich Presence needs an application registered in the
[Discord Developer Portal](https://discord.com/developers/applications). Kagero's ID is
set in [`src/release.rs`](https://github.com/senzukisu/kagero/blob/main/src/release.rs):

```rust
pub const DISCORD_APPLICATION_ID: &str = "1564753232763424869";
```

The application ID is public. **Never** put the application's client secret, bot token
or any OAuth secret into Kagero — Rich Presence doesn't need them. With an empty ID the
integration is simply turned off.

The application's **name** in the portal ("Kagero") is what Discord shows as
"Playing Kagero".

## Images (Rich Presence assets)

In the portal open the application → *Rich Presence* → *Art Assets* and upload images
(at least 512×512 PNG) with exactly these keys:

| Key           | Used as                         |
|---------------|---------------------------------|
| `kagero_logo` | Large image, always             |
| `pixel_art`   | Small image while drawing       |
| `animation`   | Small image for animations      |
| `workshop`    | Small image on the start screen |

Assets can take a few minutes to appear. Until they exist, the presence still works,
just without pictures.

## What is shown

| Kagero state            | Details line            | State line           |
|-------------------------|-------------------------|----------------------|
| Start screen            | Browsing the Workshop   | Ready to Create      |
| Single-frame document   | Creating Pixel Art      | Editing a Sprite     |
| Multi-frame document    | Animating Sprites       | Animating            |
| Animation preview       | Previewing Animation    | Animating            |

With *Show the project name* on, the state line becomes `Project: <file name>`.
With *Show what I'm doing* off, it shows only "Using Kagero". The elapsed timer counts
from the start of the Kagero session.

## Privacy

Settings → Discord. Defaults: enabled, generic activity only, **project name off**.
Kagero never sends file paths, folder names, the Windows user name, image contents,
history or anything else from the document. Turning the integration off (or
*Disconnect*) clears the activity when *Clear my status when turned off* is on.

## Checking it works

1. Start the Discord desktop app and sign in.
2. Discord → User Settings → *Activity Privacy* → enable *Share your detected
   activities with others*.
3. Start Kagero → Settings → Discord shows **Connected to Discord**.
4. Your profile shows "Kagero — Browsing the Workshop".

## Troubleshooting

| Problem                          | Fix                                                        |
|----------------------------------|------------------------------------------------------------|
| "Discord desktop app is not running" | Start Discord (the browser version has no IPC)        |
| Connected but nothing shown      | Activity sharing is off in Discord's privacy settings      |
| No images                        | Upload the assets above with the exact keys                |
| Works after restart only         | Turn on *Reconnect automatically*, or press Connect/Retry  |
| Discord runs as administrator    | Run both as the same user/elevation level                  |
