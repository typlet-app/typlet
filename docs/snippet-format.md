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

Common date formats: `%d.%m.%Y` → `23.10.2026`, `%d.%m.%Y, %H:%M` → `23.10.2026, 09:42`, `%A, %e. %B %Y` → `Friday, 23. October 2026` (localised names follow with the engine).

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

## More examples

- [`examples/email/email-signature.json`](examples/email/email-signature.json): bold, italic and a link (category `email`)
- [`examples/callback-note.json`](examples/callback-note.json): a form with text, choice and yes/no fields, a list and a cursor position

## Validation rules

A snippet is rejected if:

- its `version` is newer than the supported one,
- the trigger is empty, longer than 64 characters or contains spaces or control characters,
- it contains more than one `cursor`,
- a `text` inline contains a newline,
- a `date_time` format is invalid,
- a `field` inline references an undefined field,
- two fields share a name, or a `choice` field has no options.

## Versioning

The `version` number increases only for changes that older versions would misinterpret. Adding optional fields does not require a new version. When the version increases, Typlet migrates files on load and this document describes each step.
