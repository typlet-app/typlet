# ✅ Manual test checklist

The keyboard engine talks to real keyboards and real applications, so part of every release is tested by hand. Use a [test build](test-build.md), tick what works and open an issue for what does not, including the report from **Diagnostics → Copy report** (it contains no typed text).

Record: Typlet version, Windows or Linux version, keyboard layout (for example German QWERTZ).

## 1. First start

- [ ] The set-up assistant opens and lists five example snippets in your language.
- [ ] Page 2 shows green checks (Linux: after the one-time set-up and logging in again).
- [ ] Page 3: typing the example trigger in the test field replaces it and shows "It works!".
- [ ] Page 4: "Start with the computer" is on; after a restart Typlet is running.
- [ ] The tray icon appears next to the clock; its menu opens with a right click (Windows) or click (Linux).

## 2. Expanding in apps

Type the example triggers (`:mfg`/`:br` short text, `:adr`/`:addr` multi-line, `:sig` formatted, `:datum`/`:date` date) in each app.

| App | Short | Multi-line | Formatted | Date | Notes |
|---|---|---|---|---|---|
| Notepad / Text Editor | | | (plain) | | |
| Word / LibreOffice Writer | | | | | |
| Outlook / Thunderbird | | | | | |
| Excel / LibreOffice Calc (cell) | | | (plain only) | | |
| Chrome / Edge / Firefox (text field) | | | | | |
| Gmail or Outlook Web (compose) | | | | | |
| Teams | | | | | Line breaks must not send the message |
| WhatsApp Desktop / Web | | | | | Line breaks must not send the message |
| Slack / Discord | | | | | |
| VS Code | | | | | |
| Terminal (Windows Terminal / GNOME Terminal) | | | | | |
| Remote desktop (mstsc) | | | | | |

For every cell check:

- [ ] the trigger disappears completely,
- [ ] the text is complete, with umlauts and special characters (ä ö ü ß € @ ’),
- [ ] the cursor ends at the cursor placeholder, if the snippet has one,
- [ ] the clipboard still holds what you copied before (paste it somewhere).

## 3. Typing behaviour

- [ ] Backspace right after an expansion restores the trigger (plain-text snippets).
- [ ] Dead keys still work after Typlet started: `^` then `e` gives `ê`, `´` then `e` gives `é`.
- [ ] AltGr characters work in triggers and text (`@`, `€`, `{`).
- [ ] Clicking elsewhere in the middle of typing a trigger, then finishing it, does not expand.
- [ ] Caps Lock on: triggers typed in capitals do not expand (unless a snippet uses capitals).
- [ ] Holding Shift while finishing a trigger does not select text.
- [ ] Typing fast in a long text never expands inside normal words when triggers start with `:`.

## 4. Quick search and forms

- [ ] `Ctrl+Shift+Space` opens the search near the mouse, with the cursor in the search field.
- [ ] Typing filters instantly; umlauts and accents are ignored (`grusse` finds `Grüße`).
- [ ] Arrow keys and Enter insert into the app that was active before; Esc returns there without inserting.
- [ ] A snippet with a form (`:rr`/`:call`) opens the form; the trigger is removed.
- [ ] Ctrl+Enter or "Insert" inserts the values; Cancel types the trigger back.
- [ ] Clicking the tray icon opens the search; choosing a snippet inserts into the last app you typed in.

## 5. New snippet from selection

- [ ] Select text in any app, press `Ctrl+Alt+S`: the new-snippet window shows the text.
- [ ] Empty, spaced, too long and already used triggers show a message; Save is disabled.
- [ ] After saving, the new trigger works immediately.
- [ ] Without a selection, a notice explains what to do.

## 6. Tray menu and settings

- [ ] Pause: triggers stop expanding and the icon turns grey; Resume restores both.
- [ ] Open snippet folder shows the JSON files.
- [ ] Editing or deleting a snippet file is picked up within a second.
- [ ] A broken snippet file shows a notice; Diagnostics lists the file; other snippets keep working.
- [ ] Starting Typlet a second time opens the editor or a notice, never a second copy.
- [ ] Quit ends Typlet; the hook is gone (typing triggers does nothing).

## 7. Editor

- [ ] "Open editor" in the tray opens the editor; starting it again focuses the open window.
- [ ] Starting the editor while the background program is not running starts it.
- [ ] Create a snippet with bold text, a link, a list, a date chip, the cursor and a form field; save; it expands correctly in Word/Outlook and in Notepad.
- [ ] Changing the date format and the day shift updates the chip and the preview.
- [ ] Duplicate and conflicting triggers (`:c` and `:cc`) show warnings; a trigger with spaces cannot be saved.
- [ ] Typing a trigger inside the editor does not expand it.
- [ ] Moving a snippet to another category moves its file; deleting asks first.
- [ ] Unsaved changes: switching to another snippet asks before discarding.
- [ ] Settings: changing the language, a shortcut or an app rule takes effect in the background program without a restart.
- [ ] "Edit in the editor" after creating a snippet from a selection opens exactly that snippet.
- [ ] Import from espanso: dates, clipboard, forms and formatted text arrive; scripts and regex triggers are listed as not imported; espanso's files are unchanged.

## 8. Safety and privacy

- [ ] In a password field (Windows login form in a browser, KeePass), triggers do not expand; Diagnostics says "skipped: password field".
- [ ] `typlet.log` contains no typed text and no snippet text.
- [ ] Win+V clipboard history does not contain the pasted snippet (Windows).
- [ ] Apps running as administrator: expansion fails with a clear notice (expected for unsigned builds).

## 9. Diagnostics

- [ ] Shows keyboard, typing and app detection status.
- [ ] Recent expansions list the app and how the text was inserted.
- [ ] "Copy report" copies a report you can paste into an issue.
