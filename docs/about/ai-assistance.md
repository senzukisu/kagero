---
description: "An honest note on how Kagero is developed."
icon: robot
---

# How Kagero is made (AI assistance)

Kagero is developed by **Onibi Lab**. We want to be open about our tools:

**Yes, large language models (LLMs) are used in Kagero's development — but not for all
of the code.** Most often an LLM works as an assistant: helping with code, and helping
with building and publishing releases.

## What that means in practice

* **Help with code** — suggesting implementations, finding bugs, refactoring, writing
  tests and reviewing changes.
* **Help with publishing** — release packaging, checksums and signing scripts,
  release notes, and documentation such as this site.
* **Decisions stay with the developer** — what Kagero is, how it looks and behaves, and
  whether a change is accepted.

## How quality is kept

Every change — written by hand or with help — goes through the same checks:

* `cargo fmt --check` and `cargo clippy --all-targets`
* the automated test suite (100+ tests covering drawing, layers, animation, the project
  format, the updater and the Discord integration)
* manual testing of the app before a release

Commits made with an assistant's help are marked with a `Co-Authored-By` line in the
[git history](https://github.com/senzukisu/kagero/commits/main), so it stays transparent
which changes had help.

{% hint style="success" %}
No user data is ever sent to an AI service by Kagero. The app itself contains no AI
features and works fully offline.
{% endhint %}
