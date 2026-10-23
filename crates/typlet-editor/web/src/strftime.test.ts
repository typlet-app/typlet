// SPDX-License-Identifier: GPL-3.0-or-later
import { describe, expect, it } from "vitest";
import { shiftedToday, strftime } from "./strftime";

describe("strftime", () => {
  const date = new Date(2026, 9, 3, 9, 5, 7); // Saturday, 3 October 2026

  it("formats the editor presets like chrono", () => {
    expect(strftime("%d.%m.%Y", date)).toBe("03.10.2026");
    expect(strftime("%d.%m.%y", date)).toBe("03.10.26");
    expect(strftime("%d.%m.%Y, %H:%M", date)).toBe("03.10.2026, 09:05");
    expect(strftime("%Y-%m-%d", date)).toBe("2026-10-03");
    expect(strftime("%B %-d, %Y", date)).toBe("October 3, 2026");
    expect(strftime("%A %e %b %a %S %%", date)).toBe("Saturday  3 Oct Sat 07 %");
  });

  it("uses German names like chrono's de_DE locale", () => {
    expect(strftime("%A, %-d. %B %Y", date, "de")).toBe("Samstag, 3. Oktober 2026");
    expect(strftime("%a %b", new Date(2026, 2, 6), "de")).toBe("Fr Mär");
  });

  it("keeps unknown patterns", () => {
    expect(strftime("%Q", date)).toBe("%Q");
  });

  it("shifts by days across months", () => {
    expect(strftime("%d.%m.", shiftedToday(30, date))).toBe("02.11.");
  });
});
