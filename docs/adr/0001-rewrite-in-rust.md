# 0001. Build from scratch in Rust instead of forking espanso

- Status: Accepted
- Date: 2026-10-23

## Context

espanso is a capable open-source text expander, but it is configured through YAML files, which makes it hard for non-technical users. Community GUIs exist on top of it, yet they cannot fix problems in the engine itself (unreliable line breaks in some apps, no hotkey triggers, Wayland issues). Forking would also bind Typlet to espanso's architecture and its GPL-3.0 code base.

The process that handles keystrokes must be fast, small and safe: it runs all day, sees every key and on Windows must answer a keyboard hook within a strict time limit.

## Decision

Typlet is written from scratch, without espanso code. The implementation language is **Rust**:

- no garbage collector, so no pauses that could get a keyboard hook removed,
- memory safety for a process that handles every keystroke,
- small native binaries and good access to Win32, Linux and macOS APIs.

## Consequences

- Full freedom over architecture, data model and user experience.
- More initial work; platform input handling must be built and tested per OS.
- No espanso code may be copied. Compatibility is limited to importing espanso snippet files.
