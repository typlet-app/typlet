# typlet-import

Imports snippets from other text expanders into the [Typlet](../../README.md) format. Used by the editor's import function.

## espanso

`import_dir` reads every match file below an espanso configuration folder (`match/**/*.yml`, except files starting with `_`) and converts each trigger to one Typlet snippet. Nothing is written; the caller saves the result (the editor uses the category `espanso/<file>`).

| espanso | Typlet |
|---|---|
| `replace`, `markdown`, `html` | Text with bold, italic, underline, links, line breaks and lists |
| `$\|$` | Cursor position |
| `date` variable (`format`, `offset` in seconds) | Date placeholder (offset rounded to whole days) |
| `clipboard` variable | Clipboard placeholder |
| `echo` variable, `global_vars` | Fixed text |
| `form` + `form_fields`, `form` variables, `choice` variables | Form fields (text, multi-line, choice) |
| `label` | Name |

Skipped with a reason: regular-expression triggers, images, triggers with spaces and variable types that run programs (`shell`, `script`) or have no equivalent (`random`). `word` and `propagate_case` are imported with a warning.

```sh
cargo test -p typlet-import
```
