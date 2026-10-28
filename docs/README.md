---
description: "An independent pixel-art and sprite-animation editor by Onibi Lab."
icon: fire-flame-curved
cover: images/screenshot-onion-skin.png
coverY: 0
layout:
  cover:
    visible: true
    size: hero
---

# Welcome to Kagero

<figure><img src="images/logo.png" alt="Kagero flame logo" width="96"><figcaption></figcaption></figure>

**Kagero 陽炎** ("heat haze") is a small, fast desktop editor for pixel art and
frame-by-frame animation, written in Rust by **Onibi Lab**. It runs entirely on your
computer — no account, no server, no installer.

{% hint style="info" %}
**Current version: 0.3.0 Beta** — export & sprite workflow. Windows 10/11 x64.
[Download it from GitHub Releases](https://github.com/senzukisu/kagero/releases/latest).
{% endhint %}

## What you can do

<table data-view="cards">
<thead><tr><th></th><th></th><th data-hidden data-card-target data-type="content-ref"></th></tr></thead>
<tbody>
<tr><td><strong>Draw pixel art</strong></td><td>Pencil, eraser, fill, shapes and an eyedropper on a pixel-exact canvas.</td><td><a href="user-guide/drawing-tools.md">drawing-tools.md</a></td></tr>
<tr><td><strong>Work in layers</strong></td><td>Stack, hide, rename and reorder layers — every step undoable.</td><td><a href="user-guide/layers.md">layers.md</a></td></tr>
<tr><td><strong>Animate</strong></td><td>Frames, a timeline with thumbnails, real-time preview and onion skin.</td><td><a href="user-guide/animation.md">animation.md</a></td></tr>
<tr><td><strong>Select and transform</strong></td><td>Move, copy, paste, flip and rotate parts of a layer.</td><td><a href="user-guide/selection.md">selection.md</a></td></tr>
<tr><td><strong>Export for games</strong></td><td>PNG, JPEG, WebP, animated GIF and sprite sheets with JSON metadata, at any integer scale.</td><td><a href="user-guide/export.md">export.md</a></td></tr>
</tbody>
</table>

## Start here

1. [Install Kagero](getting-started/installation.md) — download, unzip, run.
2. [Make your first animation](getting-started/first-animation.md) — a five-minute walkthrough.
3. [Learn the workspace](getting-started/interface-tour.md) and the [shortcuts](user-guide/keyboard-shortcuts.md).

## Principles

* **Pixel-exact.** Every image pixel covers whole screen pixels at any zoom and display scale.
* **Safe with your work.** Saves never leave a half-written file; every edit can be undone.
* **Works offline.** Updates and Discord are optional extras, never requirements.
* **Private by default.** Nothing about your projects leaves your computer unless you turn it on.

{% hint style="success" %}
Kagero is open source under the MIT License. Source code and issues:
[github.com/senzukisu/kagero](https://github.com/senzukisu/kagero).
{% endhint %}
