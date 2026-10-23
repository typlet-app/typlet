// SPDX-License-Identifier: GPL-3.0-or-later
// A small strftime for showing example dates in the editor. Insertion uses
// the Rust renderer (chrono); this only covers the patterns the editor
// offers, plus common ones users type by hand.

// The same names as the Rust renderer (chrono's en_US and de_DE locales).
const NAMES = {
  en: {
    days: ["Sunday", "Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday"],
    shortDays: ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"],
    months: [
      "January", "February", "March", "April", "May", "June",
      "July", "August", "September", "October", "November", "December",
    ],
    shortMonths: ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"],
  },
  de: {
    days: ["Sonntag", "Montag", "Dienstag", "Mittwoch", "Donnerstag", "Freitag", "Samstag"],
    shortDays: ["So", "Mo", "Di", "Mi", "Do", "Fr", "Sa"],
    months: [
      "Januar", "Februar", "März", "April", "Mai", "Juni",
      "Juli", "August", "September", "Oktober", "November", "Dezember",
    ],
    shortMonths: ["Jan", "Feb", "Mär", "Apr", "Mai", "Jun", "Jul", "Aug", "Sep", "Okt", "Nov", "Dez"],
  },
} as const;

const pad = (n: number, width = 2) => String(n).padStart(width, "0");

/** Formats `date`; day and month names follow `lang` like the inserted text. */
export function strftime(format: string, date: Date, lang: "en" | "de" = "en"): string {
  const names = NAMES[lang];
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
      case "A": out += names.days[date.getDay()] ?? ""; break;
      case "a": out += names.shortDays[date.getDay()] ?? ""; break;
      case "B": out += names.months[date.getMonth()] ?? ""; break;
      case "b": out += names.shortMonths[date.getMonth()] ?? ""; break;
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
