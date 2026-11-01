# Plugins

Plugins add commands to Kagero's **Plugins** menu. A plugin is a small [Lua 5.4](https://www.lua.org/manual/5.4/) script that reads and changes the pixels of the active layer, for example to invert colors or add an outline.

## Install and turn on

1. Open **Settings → Plugins** (or **Plugins → Manage Plugins…**).
2. Click **Open Plugins Folder** and copy the plugin's folder into it. To try the bundled example instead, click **Install Example Plugin**.
3. Click **Reload**. Plugins with a problem show the reason in red.
4. Turn the plugin on. Its commands appear in the **Plugins** menu.

New plugins always start turned off.

The plugins folder is `%APPDATA%\OnibiLab\Kagero\plugins`.

## Running a command

A command works on the **active layer of the current frame**. If something is selected, it changes only the selected pixels.

- The command runs in the background, so Kagero doesn't freeze. Press **Esc** to stop it.
- When it finishes, everything it changed is applied as **one undo step**.
- If you edit the image while it runs, its result is thrown away and Kagero tells you so.

## Safety

Plugins run in a sandbox:

- **What a plugin can use:** only Lua's `table`, `string`, `math` and `utf8` libraries, plus the `kagero` API below.
- **What it can't do:** there is no `io`, `os`, `require`, `debug`, `load`, `dofile` or `setmetatable`, and no precompiled bytecode. A plugin can't read or write files, use the network or start programs.
- **Limits:** a command may use at most **64 MiB** of memory and run for at most **20 seconds**. After that it is stopped.

Even so, only turn on plugins from people you trust.

## Writing a plugin

A plugin is a folder with two files:

```
my-plugin/
├── plugin.json
└── main.lua
```

### plugin.json

```json
{
  "format": "kagero-plugin",
  "api": 1,
  "id": "yourname.my-plugin",
  "name": "My Plugin",
  "version": "1.0.0",
  "author": "Your Name",
  "description": "What it does.",
  "commands": [
    { "id": "make-red", "title": "Make It Red" }
  ]
}
```

| Field | Rules |
| --- | --- |
| `format` | Always `"kagero-plugin"`. |
| `api` | The plugin API version it was written for. Kagero 0.4 provides **1**. A higher number means "needs a newer Kagero". |
| `id` | Unique. 2–64 characters from `a-z`, `0-9`, `.`, `-` and `_`. |
| `name` | 1–48 characters. Shown in Settings and in the menu. |
| `version` | A [semantic version](https://semver.org) such as `1.0.0`. |
| `author`, `description` | Optional. |
| `commands` | 1–32 commands. Each has an `id` (same rules as the plugin id) and a `title` (1–48 characters). |

### main.lua

`main.lua` must **return a table** with one function per command id:

```lua
local commands = {}

commands["make-red"] = function()
  for y = 0, kagero.height - 1 do
    for x = 0, kagero.width - 1 do
      local r, g, b, a = kagero.get_pixel(x, y)
      if a > 0 then
        kagero.set_pixel(x, y, 255, g, b, a)
      end
    end
  end
  kagero.message("Done!")
end

return commands
```

### The `kagero` API (version 1)

Coordinates start at `0, 0` in the top-left corner. Colors are four numbers (red, green, blue, alpha) from 0 to 255.

| Name | What it does |
| --- | --- |
| `kagero.api_version` | `1`. |
| `kagero.width`, `kagero.height` | Image size in pixels. |
| `kagero.get_pixel(x, y)` | Returns `r, g, b, a`, or nothing (`nil`) outside the image. |
| `kagero.set_pixel(x, y, r, g, b [, a])` | Sets a pixel. `a` defaults to 255, and values are rounded and clamped to 0–255. Returns `false` (and changes nothing) outside the image or outside the selection. |
| `kagero.selection()` | Returns `x, y, w, h` of the box around the selection, or `nil` when nothing is selected. |
| `kagero.is_selected(x, y)` | `true` for pixels that may be changed: every pixel inside the image when nothing is selected. |
| `kagero.primary()`, `kagero.secondary()` | The current colors as `r, g, b, a`. |
| `kagero.set_primary(r, g, b [, a])` | Changes the primary color. |
| `kagero.message(text)` | Shows a message in the status bar after the command finishes. `print(...)` does the same. At most 10 messages of 200 characters are kept. |

If the script raises an error (or `error("...")` is called), nothing is changed and the message is shown in the status bar.

### Example

The bundled **Example Tools** plugin (Invert Colors, Grayscale, Outline with Primary Color) is in the source code under `examples/plugins/kagero.example-tools`. It is also installed by **Install Example Plugin**.

## Why Lua?

Rust plugins would be native DLLs. They would run with Kagero's full rights, could crash it, and would have to be built with exactly the same compiler version, because Rust has no stable binary interface. Lua was made to be embedded, can be sandboxed, and needs only a text editor to write.
