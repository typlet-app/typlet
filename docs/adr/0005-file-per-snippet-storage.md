# 0005. One JSON file per snippet, folders as categories

- Status: Accepted
- Date: 2026-10-23

## Context

Snippets must be easy to back up, sync between computers and share within a team, without running a server. A single database file (SQLite) is robust locally but corrupts or conflicts when synced through OneDrive, Dropbox or similar. A single large JSON/YAML file creates merge conflicts between unrelated changes.

## Decision

- A library is a folder. Each snippet is one UTF-8 JSON file named `<uuid>.json`.
- Sub-folders are categories.
- Files are written atomically (temporary file in the same folder, then rename).
- Loading reports broken files individually and keeps going.
- The format is versioned and specified in [snippet-format.md](../snippet-format.md).

## Consequences

- Sync and sharing work with existing tools; conflicts only affect the snippet that was edited on two machines at once.
- Users and scripts can read the data without Typlet.
- Search runs on an in-memory index built at load time; with thousands of snippets this remains fast.
- File watching is required so the background program notices changes made by the editor or by sync tools.
