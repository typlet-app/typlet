// SPDX-License-Identifier: GPL-3.0-or-later
import { readFileSync, readdirSync, statSync } from "node:fs";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";
import { fromTiptap, hasCursor, includedIds, toTiptap, usedFields } from "./convert";
import type { Snippet, TypletDocument } from "./types";

/** All documented example snippets (docs/examples), which CI also renders in Rust. */
function examples(dir = fileURLToPath(new URL("../../../../docs/examples", import.meta.url))): Snippet[] {
  return readdirSync(dir).flatMap((name) => {
    const path = join(dir, name);
    if (statSync(path).isDirectory()) return examples(path);
    return name.endsWith(".json") ? [JSON.parse(readFileSync(path, "utf8")) as Snippet] : [];
  });
}

describe("conversion between Typlet and Tiptap", () => {
  it("round-trips every documented example", () => {
    const all = examples();
    expect(all.length).toBeGreaterThanOrEqual(3);
    for (const snippet of all) {
      expect(fromTiptap(toTiptap(snippet.content))).toEqual(snippet.content);
    }
  });

  it("maps marks and placeholders", () => {
    const doc: TypletDocument = {
      blocks: [
        {
          type: "paragraph",
          content: [
            { type: "text", text: "Hi ", marks: { bold: true } },
            { type: "text", text: "there", marks: { link: "https://example.org", italic: true } },
            { type: "line_break" },
            { type: "date_time", format: "%d.%m.%Y", offset_days: -1 },
            { type: "cursor" },
            { type: "clipboard" },
            { type: "field", name: "who" },
            { type: "snippet", id: "019a0f3c-5d2e-7a10-8b3c-0a1b2c3d4e02" },
          ],
        },
      ],
    };
    const tiptap = toTiptap(doc);
    expect(tiptap.content?.[0]?.content?.[3]).toEqual({
      type: "dateTime",
      attrs: { format: "%d.%m.%Y", offsetDays: -1 },
    });
    expect(fromTiptap(tiptap)).toEqual(doc);
  });

  it("merges neighbouring text and drops unknown marks", () => {
    const doc = fromTiptap({
      type: "doc",
      content: [
        {
          type: "paragraph",
          content: [
            { type: "text", text: "a", marks: [{ type: "bold" }] },
            { type: "text", text: "b", marks: [{ type: "bold" }, { type: "strike" }] },
            { type: "text", text: "c" },
          ],
        },
      ],
    });
    expect(doc).toEqual({
      blocks: [
        {
          type: "paragraph",
          content: [
            { type: "text", text: "ab", marks: { bold: true } },
            { type: "text", text: "c" },
          ],
        },
      ],
    });
  });

  it("flattens nested lists and multi-paragraph items", () => {
    const doc = fromTiptap({
      type: "doc",
      content: [
        {
          type: "bulletList",
          content: [
            {
              type: "listItem",
              content: [
                { type: "paragraph", content: [{ type: "text", text: "one" }] },
                { type: "paragraph", content: [{ type: "text", text: "more" }] },
                {
                  type: "orderedList",
                  content: [
                    { type: "listItem", content: [{ type: "paragraph", content: [{ type: "text", text: "inner" }] }] },
                  ],
                },
              ],
            },
          ],
        },
      ],
    });
    expect(doc.blocks).toEqual([
      {
        type: "bullet_list",
        items: [
          [
            { type: "text", text: "one" },
            { type: "line_break" },
            { type: "text", text: "more" },
            { type: "line_break" },
            { type: "text", text: "inner" },
          ],
        ],
      },
    ]);
  });

  it("removes trailing empty paragraphs but keeps inner ones", () => {
    const doc = fromTiptap({
      type: "doc",
      content: [
        { type: "paragraph", content: [{ type: "text", text: "a" }] },
        { type: "paragraph" },
        { type: "paragraph", content: [{ type: "text", text: "b" }] },
        { type: "paragraph" },
      ],
    });
    expect(doc.blocks).toHaveLength(3);
    expect(fromTiptap({ type: "doc", content: [{ type: "paragraph" }] })).toEqual({ blocks: [] });
  });

  it("lists used fields and finds the cursor", () => {
    const doc: TypletDocument = {
      blocks: [
        { type: "paragraph", content: [{ type: "field", name: "b" }, { type: "field", name: "a" }] },
        { type: "bullet_list", items: [[{ type: "field", name: "b" }, { type: "cursor" }]] },
      ],
    };
    expect(usedFields(doc)).toEqual(["b", "a"]);
    expect(includedIds(doc)).toEqual([]);
    expect(
      includedIds({ blocks: [{ type: "bullet_list", items: [[{ type: "snippet", id: "x" }], [{ type: "snippet", id: "x" }]] }] }),
    ).toEqual(["x"]);
    expect(hasCursor(doc)).toBe(true);
    expect(hasCursor({ blocks: [] })).toBe(false);
  });
});
