---
description: "Show \"Working in Kagero\" on your Discord profile — privately."
icon: discord
---

# Discord Rich Presence

When the **Discord desktop app** is running on the same computer, Kagero can show what
you're doing in your Discord profile. No login, token or password is needed.

## What others see

| You are… | Discord shows |
|----------|---------------|
| On the start screen | *Browsing the Workshop* · *Ready to Create* |
| Editing a single image | *Creating Pixel Art* · *Editing a Sprite* |
| Editing an animation | *Animating Sprites* · *Animating* |
| Watching the preview | *Previewing Animation* · *Animating* |

…plus the elapsed time since you started Kagero.

## Privacy

{% hint style="success" %}
By default Kagero shares **only the general activity**. Your project name is shared
only if you turn on **Show the project name** — and even then only the file name, never
the folder, your Windows user name or anything from the image.
{% endhint %}

Settings → **Discord**:

* **Enable Discord Rich Presence** — on/off.
* **Show what I'm doing** — off shows just "Using Kagero".
* **Show the project name** — off by default.
* **Show elapsed time**.
* **Reconnect automatically** when Discord starts later.
* **Clear my status when turned off**.
* **Connect / Retry** and **Disconnect** buttons with the connection status.

## Troubleshooting

| Problem | Fix |
|---------|-----|
| *Not connected — Discord desktop app is not running* | Start the Discord app (the browser version can't be detected). |
| Connected, but nothing in the profile | Discord → User Settings → *Activity Privacy* → enable sharing your activity. |
| No pictures in the status | The Rich Presence images haven't been uploaded for the app yet; the text still works. |
| Discord runs as administrator | Run Kagero and Discord at the same permission level. |

Technical details: [Discord RPC internals](../developers/discord-rpc-internals.md).
