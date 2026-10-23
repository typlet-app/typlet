// SPDX-License-Identifier: GPL-3.0-or-later
// Snippets inside snippets: which snippets can be included without a loop,
// and which snippets use a given one. The Rust side (typlet_core::nesting)
// checks the same rules again when a snippet is saved.

import { includedIds } from "./convert";
import type { Snippet } from "./types";

/** Ids of every snippet `id` includes, directly or through others. */
function reachable(library: Snippet[], id: string, seen = new Set<string>()): Set<string> {
  const snippet = library.find((s) => s.id === id);
  for (const included of snippet ? includedIds(snippet.content) : []) {
    if (!seen.has(included)) {
      seen.add(included);
      reachable(library, included, seen);
    }
  }
  return seen;
}

/**
 * The snippets that `current` may include: every other snippet that does not
 * (directly or indirectly) include `current` itself, sorted by name.
 */
export function includable(library: Snippet[], current: Snippet): Snippet[] {
  // The draft may differ from the saved version; use the draft.
  const others = library.filter((s) => s.id !== current.id);
  return others
    .filter((s) => !reachable([...others, current], s.id).has(current.id))
    .sort((a, b) => (a.label || a.trigger).localeCompare(b.label || b.trigger));
}

/** The snippets that include the snippet `id` directly. */
export function usersOf(library: Snippet[], id: string): Snippet[] {
  return library.filter((s) => s.id !== id && includedIds(s.content).includes(id));
}
