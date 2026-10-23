// SPDX-License-Identifier: GPL-3.0-or-later
// A small strftime for showing example dates in the editor. Insertion uses
// the Rust renderer (chrono); this only covers the patterns the editor
// offers, plus common ones users type by hand.

const DAYS = ["Sunday", "Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday"];
const MONTHS = [
  "January", "February", "March", "April", "May", "June",
  "July", "August", "September", "October", "November", "December",
];

const pad = (n: number, width = 2) => String(n).padStart(width, "0");

export function strftime(format: string, date: Date): string {
  let out = "";
  for (let i = 0; i < format.length; i++) {
    const c = format[i];
    if (c !== "%") {
      out += c;
      continue;
    }
    let spec = format[++i] ?? "";
    let noPad = false;
    if (spec === "-") {
      noPad = true;
      spec = format[++i] ?? "";
    }
    const two = (n: number) => (noPad ? String(n) : pad(n));
    switch (spec) {
      case "d": out += two(date.getDate()); break;
      case "e": out += noPad ? String(date.getDate()) : String(date.getDate()).padStart(2, " "); break;
      case "m": out += two(date.getMonth() + 1); break;
      case "Y": out += String(date.getFullYear()); break;
      case "y": out += pad(date.getFullYear() % 100); break;
      case "H": out += two(date.getHours()); break;
      case "M": out += two(date.getMinutes()); break;
      case "S": out += two(date.getSeconds()); break;
      case "A": out += DAYS[date.getDay()] ?? ""; break;
      case "a": out += (DAYS[date.getDay()] ?? "").slice(0, 3); break;
      case "B": out += MONTHS[date.getMonth()] ?? ""; break;
      case "b": out += (MONTHS[date.getMonth()] ?? "").slice(0, 3); break;
      case "%": out += "%"; break;
      default: out += `%${noPad ? "-" : ""}${spec}`;
    }
  }
  return out;
}

/** Today shifted by whole days. */
export function shiftedToday(days: number, now = new Date()): Date {
  const date = new Date(now);
  date.setDate(date.getDate() + days);
  return date;
}

/** Date formats offered in the editor. */
export const DATE_PRESETS = [
  "%d.%m.%Y",
  "%d.%m.%y",
  "%d.%m.%Y, %H:%M",
  "%H:%M",
  "%Y-%m-%d",
  "%m/%d/%Y",
  "%B %-d, %Y",
] as const;
