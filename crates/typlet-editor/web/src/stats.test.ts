// SPDX-License-Identifier: GPL-3.0-or-later
import { describe, expect, it } from "vitest";
import { translate } from "./i18n";
import { activityLevel, dayCount, duration, weekColumns } from "./stats";

describe("statistics helpers", () => {
  it("puts days into Monday-to-Sunday weeks", () => {
    // 2026-10-21 is a Wednesday.
    const days = ["2026-10-21", "2026-10-22", "2026-10-23", "2026-10-24", "2026-10-25", "2026-10-26"].map((date) => ({
      date,
      count: 1,
    }));
    const weeks = weekColumns(days);
    expect(weeks).toHaveLength(2);
    expect(weeks[0]?.slice(0, 2)).toEqual([null, null]);
    expect(weeks[0]?.[2]?.date).toBe("2026-10-21");
    expect(weeks[1]?.[0]?.date).toBe("2026-10-26");
    expect(weekColumns([])).toEqual([]);
  });

  it("scales activity to the busiest day", () => {
    expect(activityLevel(0, 10)).toBe(0);
    expect(activityLevel(1, 10)).toBe(1);
    expect(activityLevel(5, 10)).toBe(2);
    expect(activityLevel(10, 10)).toBe(4);
  });

  it("formats durations and days in both languages", () => {
    const en = (key: Parameters<typeof translate>[1], vars?: Record<string, string | number>) => translate("en", key, vars);
    const de = (key: Parameters<typeof translate>[1], vars?: Record<string, string | number>) => translate("de", key, vars);
    expect(duration(en, 45)).toBe("45 min");
    expect(duration(en, 192)).toBe("3 h 12 min");
    expect(duration(de, 192)).toBe("3 Std. 12 Min.");
    expect(duration(de, 180)).toBe("3 Std.");
    expect(dayCount(en, 1)).toBe("1 day");
    expect(dayCount(de, 5)).toBe("5 Tage");
  });
});
