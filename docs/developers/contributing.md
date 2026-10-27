---
description: How to build, test and send changes.
icon: code-pull-request
---

# Contributing

Issues and pull requests are welcome at
[github.com/senzukisu/kagero](https://github.com/senzukisu/kagero).

## Build and check

```bash
cargo run                # debug build
cargo fmt --check
cargo clippy --all-targets
cargo test
```

All three checks must pass before a change is merged.

## Rules

* Read the [architecture](architecture.md) first. The UI never edits pixels directly:
  every document change goes through the `Editor` and a `Change` in the history.
* Every visible button or menu item must work — no placeholders.
* No `unwrap()`/`expect()` in code paths a user can reach; return a typed error and
  show a clear message. Tests may use `expect`.
* Validate untrusted input (files, settings, network data) before allocating memory.
* Add tests for new behavior and keep these docs in sync.
* Only add dependencies with permissive licenses and update `THIRD_PARTY_LICENSES.md`.
* Don't copy code, icons, logos or other assets from Aseprite or other editors.

## AI-assisted contributions

Using an AI assistant is fine — Kagero itself is developed with one
([how Kagero is made](../about/ai-assistance.md)). You are responsible for every line
you submit: understand it, test it, and mark assisted commits with a
`Co-Authored-By` line.

## Commit style

Short imperative subject (`Add layer opacity`), a body explaining *why* when it isn't
obvious, and an entry in `CHANGELOG.md` for user-visible changes.

By contributing you agree that your contribution is licensed under the MIT License.
