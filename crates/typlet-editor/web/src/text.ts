// SPDX-License-Identifier: GPL-3.0-or-later
// Text helpers for searching snippets.

import type { Inline, TypletDocument } from "./types";

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

/** The plain text of a document, without placeholders. */
export function previewText(doc: TypletDocument): string {
  const text = (inlines: Inline[] | undefined) =>
    (inlines ?? []).map((i) => (i.type === "text" ? i.text : " ")).join("");
  return doc.blocks
    .map((block) => (block.type === "paragraph" ? text(block.content) : block.items.map(text).join(" ")))
    .join(" ");
}
