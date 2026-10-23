// SPDX-License-Identifier: GPL-3.0-or-later
// Converts between the Typlet document format and Tiptap's JSON.
//
// Typlet documents are deliberately simple (paragraphs and one level of
// lists); Tiptap can hold more (nested lists, several paragraphs per list
// item). `fromTiptap` flattens such structures with line breaks, so the
// result can always be saved.

import type { JSONContent } from "@tiptap/core";
import type { Block, Inline, Marks, TypletDocument } from "./types";

/** Node names of the placeholder chips in the editor. */
export const NODE = {
  dateTime: "dateTime",
  cursor: "cursor",
  clipboard: "clipboard",
  field: "field",
} as const;

// ----- Typlet → Tiptap -----------------------------------------------------

function marksToTiptap(marks: Marks | undefined): JSONContent["marks"] {
  if (!marks) return undefined;
  const out: NonNullable<JSONContent["marks"]> = [];
  if (marks.bold) out.push({ type: "bold" });
  if (marks.italic) out.push({ type: "italic" });
  if (marks.underline) out.push({ type: "underline" });
  if (marks.link) out.push({ type: "link", attrs: { href: marks.link } });
  return out.length > 0 ? out : undefined;
}

function inlineToTiptap(inline: Inline): JSONContent | null {
  switch (inline.type) {
    case "text": {
      if (inline.text === "") return null;
      const node: JSONContent = { type: "text", text: inline.text };
      const marks = marksToTiptap(inline.marks);
      if (marks) node.marks = marks;
      return node;
    }
    case "line_break":
      return { type: "hardBreak" };
    case "date_time":
      return {
        type: NODE.dateTime,
        attrs: { format: inline.format, offsetDays: inline.offset_days ?? 0 },
      };
    case "cursor":
      return { type: NODE.cursor };
    case "clipboard":
      return { type: NODE.clipboard };
    case "field":
      return { type: NODE.field, attrs: { name: inline.name } };
  }
}

function inlinesToTiptap(inlines: Inline[] | undefined): JSONContent[] {
  return (inlines ?? []).map(inlineToTiptap).filter((n): n is JSONContent => n !== null);
}

function paragraph(inlines: Inline[] | undefined): JSONContent {
  const content = inlinesToTiptap(inlines);
  return content.length > 0 ? { type: "paragraph", content } : { type: "paragraph" };
}

export function toTiptap(doc: TypletDocument): JSONContent {
  const content: JSONContent[] = doc.blocks.map((block) => {
    switch (block.type) {
      case "paragraph":
        return paragraph(block.content);
      case "bullet_list":
      case "ordered_list":
        return {
          type: block.type === "bullet_list" ? "bulletList" : "orderedList",
          content: block.items.map((item) => ({
            type: "listItem",
            content: [paragraph(item)],
          })),
        };
    }
  });
  return { type: "doc", content: content.length > 0 ? content : [{ type: "paragraph" }] };
}

// ----- Tiptap → Typlet -----------------------------------------------------

function marksFromTiptap(marks: JSONContent["marks"]): Marks | undefined {
  if (!marks || marks.length === 0) return undefined;
  const out: Marks = {};
  for (const mark of marks) {
    switch (mark.type) {
      case "bold":
        out.bold = true;
        break;
      case "italic":
        out.italic = true;
        break;
      case "underline":
        out.underline = true;
        break;
      case "link": {
        const href = mark.attrs?.["href"];
        if (typeof href === "string" && href !== "") out.link = href;
        break;
      }
      default:
        // Other marks (strike, code, …) have no Typlet equivalent.
        break;
    }
  }
  return Object.keys(out).length > 0 ? out : undefined;
}

function sameMarks(a: Marks | undefined, b: Marks | undefined): boolean {
  return (
    !!a?.bold === !!b?.bold &&
    !!a?.italic === !!b?.italic &&
    !!a?.underline === !!b?.underline &&
    (a?.link ?? "") === (b?.link ?? "")
  );
}

/** Appends an inline, merging neighbouring text with equal formatting. */
function push(out: Inline[], inline: Inline): void {
  const last = out[out.length - 1];
  if (inline.type === "text" && last?.type === "text" && sameMarks(last.marks, inline.marks)) {
    last.text += inline.text;
    return;
  }
  out.push(inline);
}

function inlinesFromTiptap(nodes: JSONContent[] | undefined, out: Inline[] = []): Inline[] {
  for (const node of nodes ?? []) {
    switch (node.type) {
      case "text": {
        const marks = marksFromTiptap(node.marks);
        const lines = (node.text ?? "").split(/\r?\n/);
        lines.forEach((line, i) => {
          if (i > 0) push(out, { type: "line_break" });
          if (line !== "") push(out, marks ? { type: "text", text: line, marks } : { type: "text", text: line });
        });
        break;
      }
      case "hardBreak":
        push(out, { type: "line_break" });
        break;
      case NODE.dateTime: {
        const format = String(node.attrs?.["format"] ?? "%d.%m.%Y");
        const offset = Number(node.attrs?.["offsetDays"] ?? 0);
        push(
          out,
          offset !== 0
            ? { type: "date_time", format, offset_days: offset }
            : { type: "date_time", format },
        );
        break;
      }
      case NODE.cursor:
        push(out, { type: "cursor" });
        break;
      case NODE.clipboard:
        push(out, { type: "clipboard" });
        break;
      case NODE.field:
        push(out, { type: "field", name: String(node.attrs?.["name"] ?? "") });
        break;
      default:
        // Unknown inline containers: keep their content.
        inlinesFromTiptap(node.content, out);
        break;
    }
  }
  return out;
}

/** Flattens a list item (several paragraphs, nested lists) into one line run. */
function listItemInlines(item: JSONContent): Inline[] {
  const out: Inline[] = [];
  const add = (inlines: Inline[]) => {
    if (inlines.length === 0) return;
    if (out.length > 0) push(out, { type: "line_break" });
    for (const inline of inlines) push(out, inline);
  };
  for (const child of item.content ?? []) {
    if (child.type === "bulletList" || child.type === "orderedList") {
      for (const nested of child.content ?? []) add(listItemInlines(nested));
    } else {
      add(inlinesFromTiptap(child.content));
    }
  }
  return out;
}

export function fromTiptap(doc: JSONContent): TypletDocument {
  const blocks: Block[] = [];
  for (const node of doc.content ?? []) {
    if (node.type === "bulletList" || node.type === "orderedList") {
      const items = (node.content ?? []).map(listItemInlines).filter((item) => item.length > 0);
      if (items.length > 0) {
        blocks.push({ type: node.type === "bulletList" ? "bullet_list" : "ordered_list", items });
      }
    } else {
      blocks.push({ type: "paragraph", content: inlinesFromTiptap(node.content) });
    }
  }
  // An empty editor holds one empty paragraph; that is an empty snippet.
  while (blocks.length > 0) {
    const last = blocks[blocks.length - 1];
    if (last?.type === "paragraph" && (last.content ?? []).length === 0) blocks.pop();
    else break;
  }
  return { blocks };
}

/** Names of all fields used in a document, in order of first use. */
export function usedFields(doc: TypletDocument): string[] {
  const names: string[] = [];
  const visit = (inlines: Inline[] | undefined) => {
    for (const inline of inlines ?? []) {
      if (inline.type === "field" && !names.includes(inline.name)) names.push(inline.name);
    }
  };
  for (const block of doc.blocks) {
    if (block.type === "paragraph") visit(block.content);
    else block.items.forEach(visit);
  }
  return names;
}

/** True if the document contains a cursor placeholder. */
export function hasCursor(doc: TypletDocument): boolean {
  const check = (inlines: Inline[] | undefined) => (inlines ?? []).some((i) => i.type === "cursor");
  return doc.blocks.some((b) => (b.type === "paragraph" ? check(b.content) : b.items.some(check)));
}
