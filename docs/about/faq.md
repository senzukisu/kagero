---
description: Common questions and fixes.
icon: circle-question
---

# FAQ & troubleshooting

<details>
<summary>Windows says "Windows protected your PC" when I start Kagero.</summary>

SmartScreen warns about apps without a paid code-signing certificate. Click
**More info → Run anyway**. You can verify that the download is genuine with its
signature — see [Install](../getting-started/installation.md#verify-the-download-optional).
</details>

<details>
<summary>Kagero doesn't start / the window stays black.</summary>

Kagero needs OpenGL 2.0 or newer. Update your graphics driver. If it still fails, send
`%APPDATA%\OnibiLab\Kagero\kagero.log` with an issue report.
</details>

<details>
<summary>Can I open my Kagero 0.1 projects?</summary>

Yes. They open as a one-frame animation with all layers. Saving writes the new format,
which Kagero 0.1 can't read — keep a copy if you still use 0.1.
</details>

<details>
<summary>Is onion skin saved in my project or exported?</summary>

No. It's only a view aid. Projects and PNG exports contain only your real pixels.
</details>

<details>
<summary>Why can't I draw on a layer?</summary>

The layer is hidden (eye icon), or the animation preview is playing. Show the layer, or
press <kbd>Enter</kbd> to stop the preview.
</details>

<details>
<summary>"Install and Restart" is grey.</summary>

Installing needs an official build in a folder you can write to. Move Kagero out of
`C:\Program Files` (e.g. to `%LOCALAPPDATA%\Programs\Kagero`), or install the update
manually from the release page. See [Updates](../integrations/updates.md).
</details>

<details>
<summary>Does Kagero need the internet or a Discord account?</summary>

No. Everything works offline. Update checks and Discord Rich Presence are optional and
can be turned off in Settings.
</details>

<details>
<summary>Is Kagero related to Aseprite?</summary>

No. Kagero is an independent project by Onibi Lab. It follows the general workflow of
sprite editors, but its code, design and icons are its own.
</details>

<details>
<summary>Where do I report bugs or suggest features?</summary>

On the [GitHub issue tracker](https://github.com/senzukisu/kagero/issues). Please attach
the log file if Kagero misbehaved.
</details>
