// SPDX-License-Identifier: GPL-3.0-or-later
// Text helpers for searching snippets.

import type { Inline, Snippet, TypletDocument } from "./types";

/** Lower case without accents, so "grusse" finds "Grüße" (like typlet-core's search). */
export function normalise(text: string): string {
  return text
    .toLowerCase()
    .replace(/ß/g, "ss")
    .replace(/æ/g, "ae")
    .replace(/œ/g, "oe")
    .normalize("NFD")
    .replace(/\p{M}/gu, "");
}

/** The search words of a query. */
export function searchWords(query: string): string[] {
  return normalise(query).split(/\s+/).filter(Boolean);
}

/** True if the snippet's trigger, name or text contains every word. */
export function matchesSearch(snippet: Snippet, words: string[]): boolean {
  if (words.length === 0) return true;
  const haystack = normalise(`${snippet.trigger} ${snippet.label} ${previewText(snippet.content)}`);
  return words.every((word) => haystack.includes(word));
}

/** The plain text of a document, without placeholders. */
export function previewText(doc: TypletDocument): string {
  const text = (inlines: Inline[] | undefined) =>
    (inlines ?? []).map((i) => (i.type === "text" ? i.text : " ")).join("");
  return doc.blocks
    .map((block) => (block.type === "paragraph" ? text(block.content) : block.items.map(text).join(" ")))
    .join(" ");
}
