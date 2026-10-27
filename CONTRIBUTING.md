# Contributing to Kagero

Thanks for helping! Kagero is developed by Onibi Lab and welcomes issues and pull requests.

## Before you start

- Read [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md). The UI never edits pixels directly:
  every document change goes through `Editor` and a `Change` in the history.
- Check [docs/ROADMAP.md](docs/ROADMAP.md) for what is in scope for the next version.

## Rules

- Every visible button or menu item must work. No placeholders.
- No `unwrap()`/`expect()` in code paths a user can reach; return a typed error and show
  a clear message instead. Tests may use `expect`.
- Validate untrusted input (files, settings) before allocating memory.
- Add tests for new behavior; keep `docs/` in sync.
- Only add dependencies with permissive licenses (MIT, Apache-2.0, BSD, Zlib, ISC, …) and
  update `THIRD_PARTY_LICENSES.md`.
- Do not copy code, icons, logos or other assets from Aseprite or other editors.

## Checks

All of these must pass:

```
cargo fmt --check
cargo clippy --all-targets
cargo test
```

## Commit style

Short imperative subject (`Add layer opacity`), a body explaining *why* when it is not
obvious, and an entry in `CHANGELOG.md` for user-visible changes.

By contributing you agree that your contribution is licensed under the MIT License.
