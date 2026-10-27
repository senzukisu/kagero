# Contributing to Kagero

Thanks for helping! Kagero is developed by Onibi Lab and welcomes issues and pull requests.

## Before you start

- Read [the architecture guide](docs/developers/architecture.md). The UI never edits pixels directly:
  every document change goes through `Editor` and a `Change` in the history.
- Check [the roadmap](docs/about/roadmap.md) for what is in scope for the next version.

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

## AI-assisted contributions

Using an AI assistant is fine — Kagero itself is developed with one (see
[How Kagero is made](docs/about/ai-assistance.md)). You are responsible for every line
you submit: understand it, test it, and mark assisted commits with a `Co-Authored-By`
line.
