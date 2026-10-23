# 0007. Instant expansion, longest match, backspace undo

- Status: Accepted
- Date: 2026-10-23

## Context

Triggers can fire **instantly** (on the last character) or only after a **delimiter** such as space or Enter. With delimiters, pressing Enter in a chat app would send the message before the trigger is replaced. Instant expansion avoids that but can make short triggers shadow longer ones.

## Decision

- Expansion is instant.
- The longest matching trigger wins (`:fob` beats `fob`).
- A backspace directly after an expansion undoes it and restores the typed trigger.
- The editor warns about triggers that can never fire, for example `:c`, which fires before `:cc` can be completed.
- A prefix such as `:` is recommended in the editor and the setup assistant to avoid accidental expansions in normal text.

## Consequences

- Predictable behaviour in chat apps and forms.
- Users get immediate feedback when two triggers collide.
- Implemented and tested in [`matcher.rs`](../../crates/typlet-core/src/matcher.rs).
