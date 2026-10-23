// SPDX-License-Identifier: GPL-3.0-or-later
import { describe, expect, it } from "vitest";
import { normalise, previewText } from "./text";

describe("text helpers", () => {
  it("normalises like the core search", () => {
    expect(normalise("Grüße Æon Crème")).toBe("grusse aeon creme");
  });

  it("extracts plain text", () => {
    expect(
      previewText({
        blocks: [
          { type: "paragraph", content: [{ type: "text", text: "Hallo" }, { type: "cursor" }] },
          { type: "bullet_list", items: [[{ type: "text", text: "a" }], [{ type: "text", text: "b" }]] },
        ],
      }),
    ).toBe("Hallo  a b");
  });
});
