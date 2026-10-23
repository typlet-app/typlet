# 0003. Rich-text editor as a separate Tauri + Tiptap program

- Status: Accepted
- Date: 2026-10-23

## Context

Formatted snippets (bold, italic, links, lists) need a real rich-text editor with placeholders shown as visual chips, clean pasting from Word and good keyboard behaviour. Slint can display styled text but has no rich-text editing component. Building one natively would take months. Web editors such as Tiptap (built on ProseMirror) solve this well.

## Decision

The editor, settings and setup assistant live in a **separate program**, `typlet-editor`, built with **[Tauri](https://tauri.app)** and **[Tiptap](https://tiptap.dev)** (MIT-licensed core only). It is started on demand and exits when closed.

The editor and the background program communicate only through the snippet library on disk: the editor writes files, the background program watches the folder.

## Consequences

- Best-in-class editing without slowing down or enlarging the background program.
- The keystroke process stays free of a browser engine (see [ADR 0002](0002-native-background-process.md)).
- Two UI technologies to maintain; visual consistency comes from shared design tokens.
- On Windows the editor uses WebView2, which ships with Windows 10/11; on Linux it uses WebKitGTK.
