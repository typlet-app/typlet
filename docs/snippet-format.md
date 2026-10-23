# 📦 Snippet format

Users never need to see this format; the editor writes it. It is documented so that the data stays readable, scriptable and portable.

- **Format version:** 1
- **Encoding:** UTF-8 JSON, one snippet per file
- **File name:** `<id>.json`
- **Location:** anywhere below the library folder; sub-folders are categories
- **Reference implementation:** [`snippet.rs`](../crates/typlet-core/src/snippet.rs), [`document.rs`](../crates/typlet-core/src/document.rs)

Every file in [`examples/`](examples/) is loaded, validated and rendered by the test suite, so the examples below are guaranteed to be correct.

## Minimal example

From [`examples/signature-date.json`](examples/signature-date.json):

```json
{
  "version": 1,
  "id": "019a0f3c-5d2e-7a10-8b3c-0a1b2c3d4e01",
  "trigger": ":sig",
  "label": "Name with today's date",
  "content": {
    "blocks": [
      {
        "type": "paragraph",
        "content": [
          { "type": "text", "text": "Max Mustermann, " },
          { "type": "date_time", "format": "%d.%m.%Y" }
        ]
      }
    ]
  }
}
```

Typing `:sig` produces `Max Mustermann, 23.10.2026`.

## Snippet

| Field | Type | Required | Default | Meaning |
|---|---|---|---|---|
| `version` | integer | no | `1` | Format version. Files with a newer version are rejected rather than misread. |
| `id` | UUID string | yes | | Stable identifier, also the file name. New snippets use UUID v7. |
| `trigger` | string | yes | | What the user types. 1–64 characters, no spaces or control characters. |
| `label` | string | yes | | Human-readable name for search and the editor. |
| `content` | [Document](#document) | yes | | The text that replaces the trigger. |
| `fields` | array of [Field](#fields) | no | `[]` | Form fields asked for before insertion. |
| `enabled` | boolean | no | `true` | Disabled snippets are kept but never expand. |
| `insert_mode` | `"auto"`, `"type"`, `"paste"` | no | `"auto"` | Overrides how the text is inserted ([rules](architecture.md#insertion-rules)). |
| `hotkey` | string | no | none | A global shortcut that inserts the snippet without typing the trigger, for example `"Ctrl+Alt+M"`. See [hotkeys](#hotkeys). |

Unknown fields are ignored, so newer files with extra optional data still load in older versions.

## Document

`content` holds a list of `blocks`. Each block has a `type`:

| Block `type` | Fields | Meaning |
|---|---|---|
| `paragraph` | `content`: array of [inlines](#inlines) | A paragraph. An empty `content` is an empty line. |
| `bullet_list` | `items`: array of arrays of inlines | A bulleted list. |
| `ordered_list` | `items`: array of arrays of inlines | A numbered list. |

## Inlines

| Inline `type` | Fields | Output |
|---|---|---|
| `text` | `text`, optional `marks` | The text. May not contain newlines; use `line_break`. |
| `line_break` | | A line break inside a paragraph. |
| `date_time` | `format`, optional `offset_days` | The current date/time in [`strftime` syntax](https://docs.rs/chrono/latest/chrono/format/strftime/index.html), shifted by `offset_days` (may be negative). |
| `cursor` | | Where the text cursor ends up. At most one per snippet. |
| `clipboard` | | The clipboard text at the moment of expansion. |
| `field` | `name` | The value entered for the [field](#fields) with that name. |
| `snippet` | `id` | The snippet with that `id`, see [snippets inside snippets](#snippets-inside-snippets). |

Common date formats: `%d.%m.%Y` → `23.10.2026`, `%d.%m.%Y, %H:%M` → `23.10.2026, 09:42`, `%A, %-d. %B %Y` → `Friday, 23. October 2026`.

Day and month names (`%A`, `%a`, `%B`, `%b`) follow the interface language: with German they become `Freitag, 23. Oktober 2026`. Numbers and separators come only from the format.

### Marks

`marks` is an object with any of:

| Mark | Type | Meaning |
|---|---|---|
| `bold` | boolean | **Bold** |
| `italic` | boolean | *Italic* |
| `underline` | boolean | Underlined |
| `link` | string | Hyperlink. Only `http://`, `https://` and `mailto:` targets become links; others are rendered as plain text. |

Formatted output never contains fonts or colours, so it adopts the style of the target document.

## Fields

| Field | Type | Required | Meaning |
|---|---|---|---|
| `name` | string | yes | Internal name, unique within the snippet, referenced by `field` inlines. |
| `label` | string | yes | Label shown in the form. |
| `kind` | object | yes | One of `{"type": "text"}`, `{"type": "multiline_text"}`, `{"type": "choice", "options": [...]}`, `{"type": "yes_no"}`. |
| `default` | string | no | Pre-filled value. Used if the user leaves the field untouched. |

A field may be referenced several times; the user fills it in once.

A `yes_no` field inserts the word for *yes* or *no* in the interface language (`Ja`/`Nein` in German, `Yes`/`No` in English). Its `default` may be either language's word, `true` or `1` for yes; anything else means no. A `choice` field whose default is not one of its options starts with the first option.

## Snippets inside snippets

A `snippet` inline inserts another snippet, for example a signature at the end of several letters. Change the signature once and every letter that includes it changes too. The decision is recorded in [ADR 0016](adr/0016-snippets-inside-snippets.md).

```json
{ "type": "paragraph", "content": [{ "type": "snippet", "id": "019a0f3c-5d2e-7a10-8b3c-0a1b2c3d4e02" }] }
```

- The snippet is referenced by its `id`, so changing its trigger, name or category does not break the reference.
- **Alone in a paragraph**, the included snippet's blocks are inserted unchanged: paragraphs, lists and formatting.
- **Inside text**, its paragraphs become lines; list items become lines starting with `• ` or `1. `. Formatting is kept.
- Its **form fields** are asked in the same form as the outer snippet's. Fields with the same name are asked once and share their value; the outer snippet's label wins.
- Its **cursor** is ignored; only the outer snippet decides where the cursor ends up.
- Dates show the same moment, and the clipboard is read once.
- A **switched-off** snippet (`"enabled": false`) can still be included: its trigger does nothing, but it works as a building block.
- Snippets may be nested at most **5 levels** deep. A snippet that includes itself (directly or through others) or a snippet that no longer exists is not inserted; Typlet shows a message instead, the editor refuses to save it and the diagnostics list it.

## Hotkeys

A hotkey is written as modifiers and one key joined with `+`, for example `Ctrl+Alt+M`, `Ctrl+Shift+F5` or `Strg+Umschalt+Leertaste`.

- Modifiers: `Ctrl` (`Strg`), `Alt`, `Shift` (`Umschalt`), `Meta` (`Win`, `Super`).
- Keys: letters `A`–`Z`, digits `0`–`9`, `F1`–`F24`, `Space` (`Leertaste`), `Enter` (`Eingabe`), `Tab`, `Esc`, `Insert` (`Einfg`), `Delete` (`Entf`), `Home` (`Pos1`), `End` (`Ende`), `PageUp` (`BildAuf`), `PageDown` (`BildAb`).
- At least one of `Ctrl`, `Alt` or `Meta` is required, except for function keys.
- Files store the English form; the parser accepts both languages and ignores case and spaces.

Implemented in [`hotkey.rs`](../crates/typlet-core/src/hotkey.rs). If two snippets (or a snippet and a built-in shortcut) use the same hotkey, only the first one works.

## More examples

- [`examples/email/email-signature.json`](examples/email/email-signature.json): bold, italic, a link and a hotkey (category `email`)
- [`examples/callback-note.json`](examples/callback-note.json): a form with text, choice and yes/no fields, a list and a cursor position
- [`examples/email/thanks-for-enquiry.json`](examples/email/thanks-for-enquiry.json): a date two days ahead and the e-mail signature included as a snippet

## Validation rules

A snippet is rejected if:

- its `version` is newer than the supported one,
- the trigger is empty, longer than 64 characters or contains spaces or control characters,
- it contains more than one `cursor`,
- a `text` inline contains a newline,
- a `date_time` format is invalid,
- a `field` inline references an undefined field,
- a `snippet` inline references the snippet itself,
- two fields share a name, or a `choice` field has no options.

When it is saved and inserted, a snippet is also checked against the library: every included snippet must exist, and there must be no loop and no more than 5 levels ([`nesting.rs`](../crates/typlet-core/src/nesting.rs)).

## Versioning

The `version` number increases only for changes that older versions would misinterpret. Adding optional fields does not require a new version. When the version increases, Typlet migrates files on load and this document describes each step.
