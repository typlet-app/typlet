# 0016. Snippets inside snippets

- Status: Accepted
- Date: 2026-10-23

## Context

Many snippets share parts: the same signature under every letter, the company address in quotes and invoices, a greeting in several languages. Copying these parts means changing them in many places when a phone number changes. espanso solves this with nested matches (`type: match` variables) in its configuration files; Typlet's users should get the same without editing files.

Questions to settle:

- How does a snippet refer to another one? By trigger, by name or by id?
- What happens to the included snippet's formatting, lists, form fields and cursor?
- What happens with loops (A includes B includes A), deleted snippets and very deep chains?

## Decision

- A new inline `{"type": "snippet", "id": "…"}` refers to the included snippet by its **id** ([snippet format](../snippet-format.md#snippets-inside-snippets)). Triggers and names change; ids do not, so renaming never breaks an include. The format version stays 1: a version without this inline lists the file as unreadable instead of misreading it.
- Includes are resolved **while rendering**, with the whole library in the `RenderContext` ([`render/mod.rs`](../../crates/typlet-core/src/render/mod.rs), [`nesting.rs`](../../crates/typlet-core/src/nesting.rs)). The file stores only the reference, so a change to the included snippet applies everywhere at once.
- An include **alone in a paragraph** splices the included blocks, so paragraphs, lists and formatting survive unchanged. An include **inside text** cannot hold blocks; its paragraphs become lines and list items become `• ` or `1. ` lines, with formatting kept.
- **Form fields** of all included snippets are asked in one form. Fields are merged by name; the first definition (the outer snippet's) wins, so a shared field such as `name` is asked once.
- The **cursor** of an included snippet is dropped: there is one cursor per insertion, and it belongs to the snippet the user triggered.
- **Switched-off** snippets can be included. Their trigger is off, which makes them pure building blocks.
- **Loops, missing snippets and more than five levels** are errors. The editor refuses to save such a snippet, rendering fails with a message instead of inserting half a text, and the diagnostics list affected snippets. The editor's picker never offers a snippet that would close a loop, and deleting a snippet that others include asks first.
- The espanso import converts `match` variables into includes, linked by trigger after all files are read. Matches whose nested match is missing or loops are skipped with a reason. A trigger that already exists in the library links to the existing snippet.

## Consequences

- Rendering needs the library, not only the snippet. The background program, the editor's preview and the examples test pass it in; the quick search preview shows the included text.
- The editor's preview shows included snippets as they are saved.
- A deleted or moved-away included snippet breaks the snippets that use it. This is visible (red chip in the editor, entry in the diagnostics, message on expansion) rather than silent.
- Exported or shared single snippet files may reference snippets that are not in the receiving library; the same visible errors apply.
