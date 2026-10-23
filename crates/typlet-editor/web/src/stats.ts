// SPDX-License-Identifier: GPL-3.0-or-later
// Helpers for the statistics page.

import type { Key } from "./i18n";
import type { DayCount } from "./types";

type T = (key: Key, vars?: Record<string, string | number>) => string;

/** A `YYYY-MM-DD` date as a local date. */
export function parseDay(iso: string): Date {
  const [year, month, day] = iso.split("-").map(Number);
  return new Date(year ?? 1970, (month ?? 1) - 1, day ?? 1);
}

/**
 * Splits days (oldest first) into weeks from Monday to Sunday for the
 * activity chart. The first week is padded with `null` before its first day.
 */
export function weekColumns(days: DayCount[]): (DayCount | null)[][] {
  const first = days[0];
  if (!first) return [];
  const monday = (parseDay(first.date).getDay() + 6) % 7;
  const cells: (DayCount | null)[] = [...Array<null>(monday).fill(null), ...days];
  const weeks: (DayCount | null)[][] = [];
  for (let i = 0; i < cells.length; i += 7) weeks.push(cells.slice(i, i + 7));
  return weeks;
}

/** Colour level 0–4 of a day, relative to the busiest day. */
export function activityLevel(count: number, max: number): number {
  if (count <= 0 || max <= 0) return 0;
  return Math.min(4, Math.max(1, Math.ceil((4 * count) / max)));
}

/** "45 min", "3 h" or "3 h 12 min". */
export function duration(t: T, minutes: number): string {
  if (minutes < 60) return t("duration_minutes", { minutes });
  const hours = Math.floor(minutes / 60);
  return minutes % 60 === 0
    ? t("duration_hours_only", { hours })
    : t("duration_hours", { hours, minutes: minutes % 60 });
}

/** "1 day" or "5 days". */
export function dayCount(t: T, count: number): string {
  return count === 1 ? t("stats_day_one") : t("stats_days", { count });
}
