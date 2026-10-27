---
description: "Download, unpack and run Kagero on Windows."
icon: download
---

# Install Kagero

## Requirements

* Windows 10 or 11, 64-bit
* A graphics driver with OpenGL 2.0 or newer (any GPU from the last decade)
* About 10 MB of disk space

No installer, runtime or account is needed.

## Install

{% stepper %}
{% step %}
### Download

Open the [latest release](https://github.com/senzukisu/kagero/releases/latest) and
download `kagero-<version>-windows-x64.zip`.
{% endstep %}

{% step %}
### Unpack to a folder you own

Extract the zip, for example to `%LOCALAPPDATA%\Programs\Kagero`.

{% hint style="warning" %}
Avoid `C:\Program Files`: Kagero updates itself in place, which needs a folder your
user account can write to.
{% endhint %}
{% endstep %}

{% step %}
### Run

Start `kagero.exe`. Pin it to the taskbar or create a shortcut if you like.

Windows SmartScreen may warn that the app is unrecognized, because the executable isn't
code-signed with a paid certificate. Choose **More info → Run anyway**. The package
itself is signed — see below.
{% endstep %}
{% endstepper %}

## Verify the download (optional)

Each release ships `SHA256SUMS.txt` and a minisign signature (`.minisig`).

{% tabs %}
{% tab title="Checksum (PowerShell)" %}
```powershell
Get-FileHash .\kagero-0.2.0-windows-x64.zip -Algorithm SHA256
```

Compare the result with the line in `SHA256SUMS.txt`.
{% endtab %}

{% tab title="Signature (rsign / minisign)" %}
```powershell
rsign verify kagero-0.2.0-windows-x64.zip -P RWSXS+R3BCdmSx47V91P3O9sUYv3G3tdbivI9Dta7p0hSgnJqkRV8wDe
```

The public key is also in [`kagero.pub`](https://github.com/senzukisu/kagero/blob/main/kagero.pub).
{% endtab %}
{% endtabs %}

## Where Kagero keeps its files

| What | Where |
|------|-------|
| Settings | `%APPDATA%\OnibiLab\Kagero\config.json` |
| Log file | `%APPDATA%\OnibiLab\Kagero\kagero.log` |
| Previous version after an update | `<install folder>\.kagero-backup\` |

## Uninstall

Delete the Kagero folder, and `%APPDATA%\OnibiLab\Kagero` if you also want to remove
the settings. Your `.kgr` projects are wherever you saved them.

## Build from source

{% code title="Requires Rust stable with the MSVC toolchain" %}
```bash
git clone https://github.com/senzukisu/kagero
cd kagero
cargo run --release
```
{% endcode %}

Builds from source can check for updates but never install them; see
[Updates](../integrations/updates.md).
