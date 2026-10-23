// SPDX-License-Identifier: GPL-3.0-or-later
import { describe, expect, it } from "vitest";
import { uuidv7 } from "./uuid";

describe("uuidv7", () => {
  it("has version 7, the RFC variant and the timestamp first", () => {
    const id = uuidv7(0x019a0f3c5d2e);
    expect(id).toMatch(/^019a0f3c-5d2e-7[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/);
  });

  it("sorts by time", () => {
    expect(uuidv7(1000) < uuidv7(2000)).toBe(true);
  });
});
