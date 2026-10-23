// SPDX-License-Identifier: GPL-3.0-or-later
import { describe, expect, it } from "vitest";
import { blocksSaving, triggerIssues } from "./conflicts";

const others = new Map([
  [":c", "Short"],
  [":mfg", "Greeting"],
]);

describe("trigger checks", () => {
  it("validates the trigger itself", () => {
    expect(triggerIssues("", others)).toEqual([{ kind: "empty" }]);
    expect(triggerIssues(":a b", others)).toEqual([{ kind: "spaces" }]);
    expect(triggerIssues(":".repeat(65), others)).toEqual([{ kind: "too_long" }]);
    expect(triggerIssues("::", others)).toEqual([{ kind: "symbols_only" }]);
    expect(blocksSaving({ kind: "spaces" })).toBe(true);
    expect(blocksSaving({ kind: "duplicate", other: "x" })).toBe(false);
  });

  it("finds duplicates and shadowing like the core matcher", () => {
    expect(triggerIssues(":mfg", others)).toEqual([{ kind: "duplicate", other: "Greeting" }]);
    expect(triggerIssues(":cc", others)).toEqual([{ kind: "shadowed_by", other: ":c" }]);
    expect(triggerIssues(":xc:cz", others)).toEqual([{ kind: "shadowed_by", other: ":c" }]);
    expect(triggerIssues(":m", others)).toEqual([{ kind: "shadows", other: ":mfg" }]);
    expect(triggerIssues(":ok", others)).toEqual([]);
  });
});
