---
description: "The updater, the install helper and Discord Rich Presence: how they work and stay safe."
icon: shield-halved
---

# Services: updater & Discord

Both services are optional. Offline, with GitHub unreachable, with Discord closed or in
a development build, Kagero keeps working normally. Their public configuration lives in
[`src/release.rs`](https://github.com/senzukisu/kagero/blob/main/src/release.rs) — none
of it is secret.

| Constant | Meaning |
|----------|---------|
| `GITHUB_OWNER`, `GITHUB_REPO` | Where releases come from (`senzukisu/kagero`) |
| `UPDATE_PUBLIC_KEY` | minisign public key (the base64 line of `kagero.pub`) |
| `DISCORD_APPLICATION_ID` | The public ID of the "Kagero" Discord application |

## Updater

### Release artifacts

For version `X.Y.Z` the release tagged `vX.Y.Z` contains:

| Asset | Purpose |
|-------|---------|
| `kagero-X.Y.Z-windows-x64.zip` | The package |
| `kagero-X.Y.Z-windows-x64.zip.minisig` | minisign signature of the zip |
| `SHA256SUMS.txt` | `sha256  filename` lines |

The zip is **flat** and contains exactly `kagero.exe`, `kagero-updater.exe`,
`README.md`, `LICENSE`, `THIRD_PARTY_LICENSES.md` and `CHANGELOG.md`.

### Checking

* One request per check (`GET /repos/<owner>/<repo>/releases?per_page=20`) on a
  background thread, with timeouts; HTTPS through the OS certificate store
  (`ureq` + `native-tls`).
* Tags are compared as semantic versions (`semver`), never as strings. Drafts and
  unparsable tags are ignored; prereleases are offered only when *Include prereleases*
  is on; only versions **greater** than the running one are offered.
* Assets outside `https://github.com/<owner>/<repo>/releases/download/` are ignored.
* Startup and periodic checks (6–72 h) remember the last check, so restarts don't add
  requests. Rate limits (HTTP 403/429) show "try again in an hour".

### Download and verification

1. The package streams to `%TEMP%\kagero-update\<version>\<name>.part` (max 512 MiB) and
   must match the size GitHub reported.
2. If `SHA256SUMS.txt` exists, the SHA-256 must match.
3. The `.minisig` signature is verified with the key **compiled into Kagero**
   (`minisign-verify`, Ed25519). A checksum proves integrity; only the signature, made
   with a key that never leaves the maintainer's machine, proves authenticity.
4. Only then is `.part` renamed. Any failure deletes the partial file.

Release notes are shown as plain text; nothing from GitHub is executed or passed to a
shell.

### Installation (Windows)

Kagero never overwrites its own running executable. **Install and Restart**:

1. asks to save unsaved work;
2. extracts the verified zip to `%TEMP%\kagero-update\install-<version>\files`, rejecting
   folders, odd names, oversized files and packages without both executables;
3. starts the package's `kagero-updater.exe` with `--source` and `--target`, then exits;
4. the helper waits until `kagero.exe` can be moved, moves the files it replaces into
   `<install>\.kagero-backup\` (listed in `backup.json`), copies the new files in, and
   restores the backup if anything fails;
5. starts `kagero.exe` and writes `.kagero-backup\update.log`.

**Restore Previous Version** runs the same helper with `--restore`. Only official builds
(`KAGERO_OFFICIAL_BUILD=1`) in a writable folder can install; Kagero never asks for
administrator rights.

## Discord Rich Presence

Kagero talks to the **Discord desktop app on the same computer** through its local IPC
pipe (`\\.\pipe\discord-ipc-N`) using the
[`discord-rich-presence`](https://crates.io/crates/discord-rich-presence) crate — the
documented desktop RPC handshake and `SET_ACTIVITY`. No server, login or token.

* `build_presence` turns the app state and privacy settings into the activity.
* `PresenceClient` is a transport-independent state machine: connect, send only
  changes (at most one per 4 s), retry with back-off (5 s → 2 min), clear on disable.
  Tests drive it with a fake transport.
* `DiscordPresence` runs it on a background thread; shutting down clears the activity.

**Images** are HTTPS URLs to files in this repository (`docs/images/discord/`:
`kagero-logo.png` 512×512, `pixel-art.png`, `animation.png`, `workshop.png` 128×128),
so nothing has to be uploaded to the Developer Portal; new images appear once pushed to
`main`.

**Privacy:** by default only the generic activity is shared. With *Show the project
name* the state line becomes `Project: <file name>` — never paths, folder names, the
Windows user name or image contents. Never put the application's client secret, a bot
token or any OAuth secret into Kagero; Rich Presence doesn't need them, and an empty
application ID simply turns the integration off.

## Troubleshooting

| Symptom | What to check |
|---------|---------------|
| "couldn't reach GitHub" / "HTTP 404" | Network or proxy / a private, renamed or empty repository |
| Release shown, no Download button | No `kagero-X.Y.Z-windows-x64.zip` asset |
| "signature is missing or invalid" | Missing `.minisig`, wrong key or a modified package |
| Install disabled | Development build, no public key, or not Windows |
| Update failed | `.kagero-backup\update.log` and `%APPDATA%\OnibiLab\Kagero\kagero.log` |
| Discord connected, nothing shown | Discord → *Activity Privacy* → share activity |
| No Discord images | Discord loads them from GitHub — check the network |

Publishing a release: [Contributing → Releasing](contributing.md#releasing).
