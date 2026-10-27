---
description: "How Kagero checks for, verifies and installs new versions."
icon: arrows-rotate
---

# Updates

Kagero can update itself from the official
[GitHub Releases](https://github.com/senzukisu/kagero/releases). It's optional —
offline, Kagero simply keeps working.

## Check for updates

* **Help → Check for Updates…**, or **Settings → Updates → Check for Updates**.
* By default Kagero also checks **at startup** and **once a day** while running. Change
  this in Settings → Updates (6, 12, 24 or 72 hours, or off).
* Prereleases (versions like `0.3.0-beta.1`) are offered only if you turn on
  **Include prereleases**.

When a newer version exists, the Updates page shows its version, date, size and release
notes, with a link to the release page.

## Install an update

{% stepper %}
{% step %}
### Download

Click **Download Update**. You can cancel at any time.
{% endstep %}

{% step %}
### Automatic verification

Kagero checks the size, the SHA-256 checksum and the package's **digital signature**
against the Onibi Lab key built into Kagero. A package that fails any check is deleted
and never installed.
{% endstep %}

{% step %}
### Install and Restart

Kagero asks you to save unsaved work, closes, replaces its files, keeps a backup of the
previous version and starts again — usually in a few seconds.
{% endstep %}
{% endstepper %}

{% hint style="warning" %}
**Install and Restart** needs an official build in a folder your account can write to
(for example `%LOCALAPPDATA%\Programs\Kagero`). Builds made from source, or copies in
`C:\Program Files`, can still download updates — install them manually from the
release page.
{% endhint %}

## If something goes wrong

* A failed installation **rolls back automatically** to the previous version.
* **Settings → Updates → Restore Previous Version** brings back the version you had
  before the last update.
* Details are logged in `<install folder>\.kagero-backup\update.log`.

| Message | Meaning |
|---------|---------|
| *couldn't reach GitHub* | No internet, a proxy or a firewall. Try again later. |
| *request limit was reached* | GitHub allows about 60 checks per hour per network. Wait an hour. |
| *signature is missing or invalid* | The package is not an official release — it won't be installed. |

For maintainers: [Updater internals](../developers/updater-internals.md) and the
[release process](../developers/release-process.md).
