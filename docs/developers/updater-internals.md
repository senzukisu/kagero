---
description: "Release artifacts, version logic, verification and the install helper."
icon: shield-halved
---

# Updater

Kagero can check GitHub Releases for new versions, download them, verify them and
install them on Windows. Every step is optional: offline, with GitHub unreachable or in a
development build, Kagero keeps working normally.

## Configuration

All values live in [`src/release.rs`](https://github.com/senzukisu/kagero/blob/main/src/release.rs). None of them is secret.

| Constant              | Meaning                                                       |
|-----------------------|---------------------------------------------------------------|
| `GITHUB_OWNER`        | Account that publishes releases (`senzukisu`)                 |
| `GITHUB_REPO`         | Repository (`kagero`)                                         |
| `UPDATE_PUBLIC_KEY`   | minisign public key (base64 line of `kagero.pub`)             |
| `platform_tag()`      | `windows-x64` (also `linux-x64`, `macos-arm64` if published)  |

Only the configured repository is trusted. Assets whose download URL does not start with
`https://github.com/<owner>/<repo>/releases/download/` are ignored, and the release page
link is replaced with the configured releases page if GitHub returns anything else.

Official builds are compiled with `KAGERO_OFFICIAL_BUILD=1`. Other builds (e.g.
`cargo run`) can check and download but never install.

## Release artifacts

For version `X.Y.Z` the release tagged `vX.Y.Z` contains:

| Asset                                   | Purpose                                  |
|-----------------------------------------|------------------------------------------|
| `kagero-X.Y.Z-windows-x64.zip`          | The package (see below)                  |
| `kagero-X.Y.Z-windows-x64.zip.minisig`  | minisign signature of the zip            |
| `SHA256SUMS.txt`                        | `sha256  filename` lines for all assets  |

The zip is **flat** (no folders) and contains exactly:

```
kagero.exe  kagero-updater.exe  README.md  LICENSE  THIRD_PARTY_LICENSES.md  CHANGELOG.md
```

File names must match `[A-Za-z0-9._-]+` and must not start with a dot.

## Version logic

- Tags are parsed as semantic versions (`v` prefix optional) with the `semver` crate —
  never compared as strings (`0.10.0 > 0.9.0`).
- Drafts and unparsable tags are ignored.
- Prereleases (GitHub flag or a `-beta.1`-style suffix) are offered only when
  *Include prereleases* is on.
- Only versions **greater** than the running one are offered; older builds never are.
- If the newest release has no package for this platform, the user sees the release
  with a link to its page, and no download button.

## Checks

- Settings → Updates → **Check for Updates**, or Help → Check for Updates.
- Optional check at startup and every 6/12/24/72 hours while running (default 24 h).
  The last successful check time is stored in the settings, so restarting Kagero does
  not cause extra requests.
- One request per check (`GET /repos/<owner>/<repo>/releases?per_page=20`), on a
  background thread, with connect/response timeouts. HTTP 403/429 with an exhausted rate
  limit shows "try again in an hour". Network errors show a short message and nothing
  else changes.
- HTTPS uses the operating system's TLS stack and certificate store (SChannel on
  Windows) through `ureq` + `native-tls`.

## Download and verification

1. The package streams to `%TEMP%\kagero-update\<version>\<name>.part` with progress and
   a Cancel button. Size is limited (512 MiB) and must equal the size GitHub reported.
2. If `SHA256SUMS.txt` is published, the package's SHA-256 must match its line.
3. The `.minisig` signature is verified with the public key **compiled into Kagero**
   (`minisign-verify`, Ed25519). A checksum downloaded from the same place as the file
   proves integrity, not authenticity — only the signature, made with a key that never
   leaves the maintainer's machine, proves the package is an official release.
4. Only then is `.part` renamed to the final name. Any failure deletes the partial file.

If no public key is configured, the download is kept but marked *not verified* and
**Install and Restart** stays disabled.

Release notes are shown as plain text only; nothing from GitHub's response is ever
executed or passed to a shell.

## Installation (Windows)

Kagero never overwrites its own running executable.

1. *Install and Restart* asks to save unsaved work first.
2. The verified zip is extracted to `%TEMP%\kagero-update\install-<version>\files`.
   Directories, nested paths, odd names, oversized files and packages without both
   executables are rejected.
3. The package's `kagero-updater.exe` is copied next to that folder and started with
   two local paths (`--source`, `--target`). Kagero exits.
4. The helper waits (up to 60 s) until `kagero.exe` can be moved — i.e. Kagero has
   exited — then moves every file it will replace into `<install>\.kagero-backup\` and
   records them in `backup.json`.
5. It copies the new files in. If anything fails, it restores the backup and removes
   files the update added.
6. It starts `kagero.exe` (new or restored) and writes `.kagero-backup\update.log`.

Kagero must be installed in a folder the user can write to (e.g. extracted under
`%LOCALAPPDATA%\Programs\Kagero`). If the folder needs administrator rights, Kagero says so
and suggests downloading manually; it never asks for elevation.

### Restoring the previous version

Settings → Updates → **Restore Previous Version** (shown when a backup exists) runs the
same helper with `--restore`.

## Troubleshooting

| Symptom                                   | What to check                                         |
|-------------------------------------------|-------------------------------------------------------|
| "couldn't reach GitHub"                   | Internet/proxy/firewall; try again later              |
| "request limit was reached"               | GitHub allows 60 anonymous API calls/hour per IP      |
| "HTTP 404"                                | Repository is private, renamed, or has no releases    |
| Release shown but no Download button      | No `kagero-X.Y.Z-windows-x64.zip` asset               |
| "signature is missing or invalid"         | Missing `.minisig`, wrong key, or a modified package  |
| Install button disabled                   | Development build, no public key, or not Windows      |
| Update failed                             | `.kagero-backup\update.log` in the install folder; logs in `%APPDATA%\OnibiLab\Kagero\kagero.log` |

## Publishing an update

See [Release process](release-process.md).
