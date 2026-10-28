---
description: "General, Updates and Discord settings."
icon: gear
---

# Settings

Open **File → Settings…** or press <kbd>Ctrl</kbd>+<kbd>,</kbd>. Changes apply immediately
and are saved automatically.

{% tabs %}
{% tab title="General" %}
**Workspace**

* Show the timeline (<kbd>F7</kbd>)
* Show the colors and layers panel (<kbd>F8</kbd>)
* Pixel grid when zoomed in (<kbd>Ctrl</kbd>+<kbd>G</kbd>)
* **Pixel font (experimental)** — uses Kagero Pixel, Kagero's own bitmap font, for the
  whole interface. Off by default (the classic font).
* **Reduce motion** — turns off the pulsing preview indicator and eased transitions

**Onion skin**

* Show neighbor frames (<kbd>O</kbd>), previous (warm tint) and/or next (cool tint)
* Opacity 5–90 %
{% endtab %}

{% tab title="Updates" %}
* Current version and build type
* **Check for Updates**, download with progress, **Install and Restart**
* Check at startup, check periodically (6/12/24/72 hours)
* Include prereleases (betas)
* Notify me when an update is found
* **Restore Previous Version** after an update

See [Updates](../integrations/updates.md).
{% endtab %}

{% tab title="Discord" %}
* Enable Discord Rich Presence
* Connection status, **Connect / Retry**, **Disconnect**
* What to share: activity, project name (off by default), elapsed time
* Reconnect automatically, clear the status when turned off

See [Discord Rich Presence](../integrations/discord.md).
{% endtab %}
{% endtabs %}

## Where settings are stored

`%APPDATA%\OnibiLab\Kagero\config.json` — recent files, palette, layout, onion skin,
preview timing, update and Discord preferences. Deleting the file resets everything to
defaults; a damaged file is ignored and replaced.
