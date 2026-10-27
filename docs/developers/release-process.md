---
description: "How official releases are built, signed and published."
icon: box-archive
---

# Release Process

Releases are published on
[github.com/senzukisu/kagero/releases](https://github.com/senzukisu/kagero/releases).
The process is scripted so every release is built, signed and checksummed the same way.

## One-time setup: signing key

Kagero verifies update packages with a [minisign](https://jedisct1.github.io/minisign/)
Ed25519 key. The **public** key is committed (`kagero.pub`) and compiled into Kagero
(`UPDATE_PUBLIC_KEY` in `src/release.rs`). The **secret** key stays on the maintainer's
machine and is never committed.

```
cargo install rsign2
rsign generate -p kagero.pub -s %USERPROFILE%\.kagero-release\kagero.key
```

- Protect the secret key with a password when prompted (or `-W` for an unencrypted
  key on a machine you trust) and back it up offline.
- Copy the second line of `kagero.pub` into `UPDATE_PUBLIC_KEY`.
- If the secret key is lost or leaked, generate a new pair, ship the new public key in
  a release that users install manually, and announce it in the release notes —
  installed copies only trust the key they were built with.

## Checklist for a release

1. Update `version` in `Cargo.toml`, `APP_CHANNEL` in `src/lib.rs` if needed, and
   `CHANGELOG.md`.
2. Run the checks:
   ```
   cargo fmt --check
   cargo clippy --all-targets
   cargo test
   ```
3. Smoke-test a release build manually (see [Testing](testing.md)).
4. Package and sign:
   ```
   powershell -ExecutionPolicy Bypass -File scripts\package-release.ps1
   ```
   This builds with `KAGERO_OFFICIAL_BUILD=1`, creates
   `dist\kagero-X.Y.Z-windows-x64.zip`, signs it (`.minisig`), verifies the signature and
   writes `SHA256SUMS.txt`.
5. Commit, tag and push:
   ```
   git tag -a vX.Y.Z -m "Kagero X.Y.Z"
   git push origin main --tags
   ```
6. Create the GitHub release for the tag:
   - title `Kagero X.Y.Z <Channel>`;
   - notes: highlights, the CHANGELOG section, known limitations, install steps;
   - mark **pre-release** only for versions with a prerelease suffix (e.g.
     `v0.3.0-beta.1`): the updater offers those only to users who enabled
     *Include prereleases*. Plain versions such as `v0.2.0` (even if labeled "Beta")
     are normal releases;
   - attach the three files from `dist\` exactly as named.
7. Verify end to end with Kagero's own updater code (network):
   ```
   cargo test --test updater live_github -- --ignored --nocapture
   ```
   Then download the zip from the release page, check
   `rsign verify -p kagero.pub -x <zip>.minisig <zip>`, run it, and use
   Settings → Updates → Check for Updates from the previous version.

## Installing (for users)

1. Download `kagero-X.Y.Z-windows-x64.zip` from the releases page.
2. Extract it to a folder you own, e.g. `%LOCALAPPDATA%\Programs\Kagero`.
3. Run `kagero.exe`. Later updates install from Settings → Updates.

Never put tokens, passwords or the secret key into the repository, scripts or release
notes.
