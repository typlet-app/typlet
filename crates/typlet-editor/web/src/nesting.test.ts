// SPDX-License-Identifier: GPL-3.0-or-later
import { describe, expect, it } from "vitest";
import { includable, usersOf } from "./nesting";
import type { Inline, Snippet } from "./types";

function snippet(id: string, ...content: Inline[]): Snippet {
  return { id, trigger: `:${id}`, label: id, content: { blocks: [{ type: "paragraph", content }] } };
}
const include = (id: string): Inline => ({ type: "snippet", id });

describe("snippets inside snippets", () => {
  it("offers only snippets that do not lead back to the current one", () => {
    const sig = snippet("sig", { type: "text", text: "Anna" });
    const mail = snippet("mail", include("sig"));
    const letter = snippet("letter", include("mail"));
    const library = [sig, mail, letter];
    // The signature cannot include the mail or the letter: both contain it.
    expect(includable(library, sig).map((s) => s.id)).toEqual([]);
    expect(includable(library, letter).map((s) => s.id)).toEqual(["mail", "sig"]);
    // The unsaved draft counts, not the saved version.
    const draft = snippet("sig", include("letter"));
    expect(includable(library, draft).map((s) => s.id)).toEqual([]);
  });

  it("finds the snippets that use one", () => {
    const sig = snippet("sig");
    const library = [sig, snippet("mail", include("sig"), include("sig")), snippet("other")];
    expect(usersOf(library, "sig").map((s) => s.id)).toEqual(["mail"]);
  });
});
