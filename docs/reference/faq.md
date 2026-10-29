---
description: Common questions and fixes.
icon: circle-question
---

# FAQ & troubleshooting

<details>

<summary>Windows says "Windows protected your PC" when I start Kagero.</summary>

SmartScreen warns about apps without a paid code-signing certificate. Click **More info → Run anyway**. You can check that the download is genuine with its signature — see [Verify the download](../getting-started/installation.md#verify-the-download).

</details>

<details>

<summary>Kagero doesn't start, or the window stays black.</summary>

Kagero needs OpenGL 2.0 or newer — update your graphics driver. If it still fails, attach `%APPDATA%\OnibiLab\Kagero\kagero.log` to an issue.

</details>

<details>

<summary>Why can't I draw?</summary>

The active layer is hidden (eye icon), or the animation preview is playing. Show the layer, or press <kbd>Enter</kbd> to stop the preview. The status bar says which.

</details>

<details>

<summary>Where is the Export button?</summary>

<kbd>Ctrl</kbd>+<kbd>Shift</kbd>+<kbd>E</kbd> opens the Export dialog; **Export** is at its bottom left. On small screens the options scroll, and the button stays visible (0.3.7 and newer).

</details>

<details>

<summary>My GIF colors or transparency look different.</summary>

GIF has at most 255 colors plus one transparent color, and no semi-transparency. See [Animated GIF](../export/export/animated-export.md#colors-and-transparency) for the options that control both.

</details>

<details>

<summary>Is onion skin saved in my project or exported?</summary>

No. It's only a drawing aid; projects and exports contain only your real pixels.

</details>

<details>

<summary>"Install and Restart" is grey.</summary>

Installing needs an official build in a folder you can write to. Move Kagero out of `C:\Program Files` (for example to `%LOCALAPPDATA%\Programs\Kagero`), or install the update by hand. See [Install & update](../getting-started/installation.md#update).

</details>

<details>

<summary>The text is too small / I'd like the pixel font.</summary>

The interface uses Windows display scaling. Kagero's own bitmap font can be turned on in **Settings → General → Pixel font (experimental)**.

</details>

<details>

<summary>Does Kagero need the internet or a Discord account?</summary>

No. Everything works offline; update checks and Discord Rich Presence are optional.

</details>

<details>

<summary>Is Kagero related to Aseprite?</summary>

No. Kagero is an independent project by Onibi Lab. It follows the general workflow of sprite editors, but its code, design, icons and font are its own.

</details>

<details>

<summary>Where do I report bugs or suggest features?</summary>

On the [GitHub issue tracker](https://github.com/senzukisu/kagero/issues). Please attach the log file if Kagero misbehaved.

</details>
