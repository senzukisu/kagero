---
description: "Build Kagero, run the tests, follow the rules, and publish a release."
icon: code-pull-request
---

# Contributing

Issues and pull requests are welcome at
[github.com/senzukisu/kagero](https://github.com/senzukisu/kagero).

## Build

Requires Rust stable (1.85+) with the MSVC toolchain on Windows.

```bash
git clone https://github.com/senzukisu/kagero
cd kagero
cargo run                # debug build
cargo build --release    # target\release\kagero.exe
```

Builds from source can check for updates but never install them.

## Checks

All three must pass before a change is merged:

```bash
cargo fmt --check
cargo clippy --all-targets
cargo test
```

The tests need no network, no Discord and no window: GitHub and Discord are faked, and
UI pieces (shortcuts, the Export dialog) run against a headless egui context.

| Area | Where | Covers |
|------|-------|--------|
| Pixel engine | `src/core/*`, `tests/document.rs` | compositing, bounds, size and memory limits, lines and fills, selection blocks |
| Editing | `tests/editor.rs`, `tests/history.rs`, `tests/selection.rs` | every tool, one stroke = one undo step, layers, history limits, selection ops with undo |
| Animation | `tests/animation.rs`, `src/core/animation.rs`, `onion_skin.rs` | frame operations and undo, durations, playback timing, onion skin never touching pixels |
| Files | `tests/file_io.rs`, `tests/palette.rs` | `.kgr` round trips and damaged files, v1 migration, PNG, palette files, settings migration |
| Export | `tests/export.rs`, `src/export/*` | pixel-exact PNG, JPEG, WebP, GIF timing/colors/transparency, sheet layouts, metadata, size limits, cancel |
| UI | `src/ui/canvas.rs`, `shortcuts.rs`, `export_dialog.rs`, `src/theme.rs` | pixel mapping, shortcuts vs text fields, dialog requests, font coverage |
| Services | `tests/updater.rs`, `tests/discord.rs` | version choice, verification, install and rollback; presence privacy and reconnects |

Performance benchmark (ignored by default):
`cargo test --release --test performance -- --ignored --nocapture`.

Before a release, also try the build by hand: draw, undo, layers, frames and preview,
save and reopen, every Export tab, selection, palettes, the title bar (drag, double-click,
edge resize) and Settings.

## Rules

* Read the [architecture](architecture.md) first. The UI never edits pixels directly:
  every document change goes through the `Editor` and a `Change` in the history.
* Every visible button or menu item must work — no placeholders.
* No `unwrap()`/`expect()` in code a user can reach; return a typed error and show a
  clear message.
* Validate untrusted input (files, settings, network data) before allocating memory.
* Add tests for new behavior and keep these docs in sync.
* Only add dependencies with permissive licenses, and update `THIRD_PARTY_LICENSES.md`.
* Don't copy code, icons, logos or other assets from Aseprite or other editors.
* Using an AI assistant is fine ([how Kagero is made](../about/ai-assistance.md)) — you
  are responsible for every line you submit: understand it and test it.

**Commits:** a short imperative subject (`Add layer opacity`), a body explaining *why*
when it isn't obvious, and a `CHANGELOG.md` entry for user-visible changes. By
contributing you agree that your contribution is licensed under the MIT License.

## Releasing

Releases are signed with a [minisign](https://jedisct1.github.io/minisign/) key. The
public key is committed (`kagero.pub`) and compiled into Kagero (`UPDATE_PUBLIC_KEY`);
the secret key stays on the maintainer's machine.

<details>

<summary>One-time setup: create the signing key</summary>

```
cargo install rsign2
rsign generate -p kagero.pub -s %USERPROFILE%\.kagero-release\kagero.key
```

Protect the key with a password (or `-W` on a machine you trust) and back it up
offline. Copy the second line of `kagero.pub` into `UPDATE_PUBLIC_KEY`. If the key is
ever lost or leaked, generate a new pair and ship it in a release users install by hand
— installed copies only trust the key they were built with.

</details>

{% stepper %}
{% step %}
### Version and changelog

Update `version` in `Cargo.toml` (always `X.Y.Z` — the updater compares semantic
versions), `APP_CHANNEL` in `src/lib.rs` if needed, and `CHANGELOG.md`. Run the checks.
{% endstep %}

{% step %}
### Package and sign

```
powershell -ExecutionPolicy Bypass -File scripts\package-release.ps1
```

Builds with `KAGERO_OFFICIAL_BUILD=1`, creates `dist\kagero-X.Y.Z-windows-x64.zip`
(the two executables and their `SHA256SUMS.txt`), signs and verifies it, and writes the
release's own `SHA256SUMS.txt` for the zip and its signature.
{% endstep %}

{% step %}
### Tag and publish

```
git tag -a vX.Y.Z -m "Kagero X.Y.Z"
git push origin main --tags
```

Create the GitHub release `Kagero X.Y.Z <Channel>` with highlights, the changelog
section, known limitations and install steps, and attach the three files from `dist\`.
Mark it **pre-release** only for versions with a suffix such as `0.4.0-beta.1`.
{% endstep %}

{% step %}
### Verify

```
cargo test --test updater live_github -- --ignored --nocapture
```

Then download the release, check `rsign verify -p kagero.pub -x <zip>.minisig <zip>`,
and update an older copy through Settings → Updates.
{% endstep %}
{% endstepper %}

{% hint style="danger" %}
Never put tokens, passwords or the secret key into the repository, scripts or release
notes.
{% endhint %}
