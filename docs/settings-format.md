# ⚙️ Settings file

Users change settings in the tray menu and in the editor; neither needs this file. It is documented because the background program and the editor share it: whoever saves it, the other one notices the change and reloads.

- **Location:** `%APPDATA%\Typlet\config\settings.json` (Windows), `~/.config/typlet/settings.json` (Linux)
- **Format:** UTF-8 JSON, written atomically
- **Reference implementation:** [`settings.rs`](../crates/typlet-core/src/settings.rs)

A missing file or missing fields mean defaults. A file that cannot be read is reported once and the defaults are used until it is fixed; Typlet never overwrites it on its own.

## Example

```json
{
  "version": 1,
  "language": "system",
  "paused": false,
  "start_with_system": true,
  "first_run_complete": true,
  "notifications": true,
  "search_hotkey": "Ctrl+Shift+Space",
  "selection_hotkey": "Ctrl+Alt+S",
  "app_rules": [
    { "app": "game.exe", "enabled": false },
    { "app": "crm.exe", "newline": "shift_enter", "restore_delay_ms": 800 }
  ],
  "check_for_updates": true,
  "statistics": true
}
```

## Fields

| Field | Type | Default | Meaning |
|---|---|---|---|
| `version` | integer | `1` | Settings format version |
| `language` | `"system"`, `"de"`, `"en"` | `"system"` | Interface language; `system` follows the operating system |
| `paused` | boolean | `false` | No snippets expand while `true` |
| `start_with_system` | boolean | `true` | Start Typlet at login (release builds only) |
| `first_run_complete` | boolean | `false` | The set-up assistant was finished; until then it opens at start |
| `notifications` | boolean | `true` | Show a message when inserting fails or no text was selected. Problems are always recorded in diagnostics. |
| `search_hotkey` | [hotkey](snippet-format.md#hotkeys) or `null` | `"Ctrl+Shift+Space"` | Opens the quick search; `null` turns it off |
| `selection_hotkey` | hotkey or `null` | `"Ctrl+Alt+S"` | Creates a snippet from selected text; `null` turns it off |
| `library_path` | string | not set | A different snippet folder, for example a synced folder |
| `app_rules` | array of [app rules](#app-rules) | `[]` | Behaviour per application |
| `check_for_updates` | boolean | `true` | Look for a new version once a day. The editor checks; the background program never connects to the network ([ADR 0015](adr/0015-automatic-updates.md)) |
| `statistics` | boolean | `true` | Count inserted snippets for the statistics in `stats.json`: numbers only, never text ([ADR 0017](adr/0017-local-statistics.md)) |

## App rules

| Field | Type | Meaning |
|---|---|---|
| `app` | string | Executable name, case-insensitive, for example `excel.exe` |
| `enabled` | boolean | `false` turns Typlet off in this app (default `true`) |
| `insert_mode` | `"auto"`, `"type"`, `"paste"` | Forces how text is inserted |
| `newline` | `"enter"`, `"shift_enter"` | How line breaks are typed |
| `plain_only` | boolean | Paste text without formatting |
| `restore_delay_ms` | integer | Milliseconds before the clipboard is restored (default 300) |

Built-in rules make common apps work out of the box; a user rule for the same app overrides them field by field:

| Apps | Built-in rule |
|---|---|
| `excel.exe` | `plain_only` (pasted HTML would split into cells) |
| `ms-teams.exe`, `teams.exe`, `whatsapp.exe`, `whatsapp.root.exe`, `slack.exe`, `discord.exe`, `signal.exe`, `telegram.exe` | `newline: shift_enter` (Enter would send the message) |
| `chrome.exe`, `msedge.exe`, `firefox.exe`, `brave.exe`, `opera.exe` | `restore_delay_ms: 600` |
| `mstsc.exe`, `msrdc.exe`, `wfica32.exe`, `cdviewer.exe` | `restore_delay_ms: 1200` (remote desktops) |

App rules need the focused app, which is not available on GNOME Wayland; there all apps use the defaults.
