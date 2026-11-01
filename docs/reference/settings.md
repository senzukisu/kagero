---
description: "General (with autosave), Shortcuts, Updates, Discord, Experimental, Plugins and Themes."
icon: gear
---

# Settings

**File → Settings…** or <kbd>Ctrl</kbd>+<kbd>,</kbd>. Pages are grouped in the side menu
— **App** (General, Shortcuts, Updates), **Integrations** (Discord) and **Labs**
(Experimental, Plugins, Themes). Changes apply immediately and are saved automatically.

{% tabs %}
{% tab title="General" %}
<figure><img src="../.gitbook/assets/settings-general.png" alt="Settings, General page: workspace and onion skin options"><figcaption></figcaption></figure>

* Show the timeline (<kbd>F7</kbd>) and the colors & layers panel (<kbd>F8</kbd>)
* Pixel grid when zoomed in (<kbd>Ctrl</kbd>+<kbd>G</kbd>)
* **Reduce motion** — turns off the pulsing preview badge and eased transitions
* **Autosave & recovery**: see [Autosave and recovery](#autosave-and-recovery) below.
* **Onion skin** — show neighbor frames (<kbd>O</kbd>), previous (warm) and/or next
  (cool), opacity 5–90 %
{% endtab %}

{% tab title="Shortcuts" %}
<figure><img src="../.gitbook/assets/settings-shortcuts.png" alt="Settings, Shortcuts page: actions grouped by Files, Edit, Tools and more, each with its key, a restore and a remove button"><figcaption></figcaption></figure>

See [Changing shortcuts](#changing-shortcuts) below.
{% endtab %}

{% tab title="Updates" %}
<figure><img src="../.gitbook/assets/settings-updates.png" alt="Settings, Updates page"><figcaption></figcaption></figure>

* Current version and build type, **Check for Updates**, download with progress,
  **Install and Restart**
* Check at startup and periodically (6/12/24/72 hours)
* Include prereleases (betas), notify me when an update is found
* **Restore Previous Version** after an update

How updating works: [Install & update](../getting-started/installation.md#update).
{% endtab %}

{% tab title="Discord" %}
<figure><img src="../.gitbook/assets/settings-discord.png" alt="Settings, Discord page: Rich Presence connection and privacy options"><figcaption></figcaption></figure>

See [Discord Rich Presence](#discord-rich-presence) below.
{% endtab %}

{% tab title="Experimental" %}
<figure><img src="../.gitbook/assets/settings-experimental.png" alt="Settings, Experimental page: pixel font, pixel-perfect pencil and tiled view"><figcaption></figcaption></figure>

See [Experimental features](#experimental-features) below.
{% endtab %}
{% endtabs %}

## Changing shortcuts

Every shortcut can be changed in **Settings → Shortcuts**.

1. Click the key next to an action — it shows **Press keys…**.
2. Press the new combination, e.g. <kbd>Ctrl</kbd>+<kbd>Alt</kbd>+<kbd>P</kbd>.
   <kbd>Esc</kbd> cancels.

* If the key already belongs to another action, it moves to the new one and the page
  says which action lost it.
* **↺** restores one default, **×** removes a key, **Reset all** restores every default.
* **Search** filters by action name or key.
* Menus and tooltips always show your current keys.

Some keys can't be changed: <kbd>Ctrl</kbd>+<kbd>C</kbd>/<kbd>X</kbd>/<kbd>V</kbd>
(Windows sends them as clipboard actions), <kbd>Esc</kbd>, <kbd>Space</kbd> (hold to
pan) and the arrow keys (nudge the selection or change frames). They're listed under
*Fixed keys*. Your changes are saved in the settings file; only the differences from
the defaults are stored.

## Experimental features

These features are switched on only in **Settings → Experimental**. They're still being
tested and may change in a later version; your projects are never affected.

| Feature | What it does |
|---------|--------------|
| **Pixel font** | Kagero Pixel, Kagero's own bitmap font, for the whole interface — always drawn on whole screen pixels. |
| **Pixel-perfect pencil** | 1 px Pencil and Eraser strokes skip the doubled pixel in "L" corners, so freehand lines stay one pixel thin. |
| **Tiled view** | Shows the image repeated around itself, dimmed, to check seamless tiles and patterns. View only — the copies are never drawn on or exported. |

<figure><img src="../.gitbook/assets/tiled-view.png" alt="Tiled view: the sprite in the middle with dimmed copies repeated around it"><figcaption><p>Tiled view: the image in the middle, dimmed copies around it.</p></figcaption></figure>

## Autosave and recovery

While a document has unsaved changes, Kagero writes a copy of it every **2 minutes**.
You can change the interval (1–60 minutes) or turn autosave off in Settings → General.

* Copies go to `%APPDATA%\OnibiLab\Kagero\recovery`, never next to or over your
  project (**Open Recovery Folder** shows them).
* A copy is deleted when you save, or when you close the document on purpose.
* If Kagero closes unexpectedly, the next start lists every copy with its name, original
  path and time:
  * **Open Recovered** opens it as an unsaved document;
  * **Open Original** opens the project file;
  * **Delete Copy** throws it away;
  * **Later** keeps the copies.
* **File → Recover Projects…** shows the list again any time. Copies of another
  Kagero that is still running are never offered.
* At most 10 copies and 512 MB are kept; the oldest go first.

Autosave lowers the risk of losing work, but it can't promise recovery after every
possible failure, so keep saving your projects.

## Plugins and themes

* **Plugins** add commands written in Lua to the **Plugins** menu. They run in a
  sandbox and start turned off. See [Plugins](../plugins/README.md).
* **Themes** change Kagero's colors: four built-in themes, live preview, and your own
  themes as files. See [Themes](themes.md).

{% tabs %}
{% tab title="Plugins" %}
<figure><img src="../.gitbook/assets/settings-plugins.png" alt="Settings, Plugins page: the safety note and the installed Example Tools plugin with its switch" width="560"><figcaption></figcaption></figure>
{% endtab %}

{% tab title="Themes" %}
<figure><img src="../.gitbook/assets/settings-themes.png" alt="Settings, Themes page: theme cards with color strips and Preview and Export buttons" width="560"><figcaption></figcaption></figure>
{% endtab %}
{% endtabs %}

## Discord Rich Presence

When the **Discord desktop app** runs on the same computer, Kagero can show what you're
doing in your Discord profile — no login, token or password.

| You are… | Discord shows |
|----------|---------------|
| On the start screen | *Browsing the Workshop* · *Ready to Create* |
| Editing a single image | *Creating Pixel Art* · *Editing a Sprite* |
| Editing an animation | *Animating Sprites* · *Animating* |
| Watching the preview | *Previewing Animation* · *Animating* |
| In the Export dialog | *Exporting Sprites* · *Getting game-ready* |

…with the Kagero flame, an activity icon, the elapsed time and a **Get Kagero** button
that links to the download page.

{% hint style="success" %}
**Private by default.** Kagero shares only the general activity. The project name is
shared only if you turn on **Show the project name** — and then only the file name,
never the folder, your Windows user name or anything from the image.
{% endhint %}

Options: enable/disable, show what I'm doing, show the project name, show elapsed time,
show the "Get Kagero" button, reconnect automatically, clear my status when turned off,
and **Connect / Retry** / **Disconnect** with a live connection status.

| Problem | Fix |
|---------|-----|
| *Not connected — Discord desktop app is not running* | Start the Discord app; the browser version can't be detected |
| *Discord didn't answer in time* | Discord is busy; Kagero retries by itself, or press **Connect / Retry** |
| Connected, but nothing in the profile | Discord → User Settings → *Activity Privacy* → share your activity |
| Discord runs as administrator | Run Kagero and Discord at the same permission level |

## Where settings are stored

`%APPDATA%\OnibiLab\Kagero\config.json` — recent files, palettes, layout, onion skin,
preview timing, export, shortcuts, experimental, autosave, theme, enabled plugins,
update and Discord preferences. Deleting it resets everything; a damaged file is ignored
and replaced with defaults.

Next to it are the `recovery`, `themes` and `plugins` folders. Uninstalling Kagero keeps
all of them.
