// SPDX-License-Identifier: GPL-3.0-or-later
// Demo mode: runs the editor in a normal browser with sample snippets and no
// Rust backend. Used for screenshots (`npm run screenshots`) and for working
// on the interface without starting Tauri. Only included when the build sets
// VITE_DEMO=1.

import { mockIPC, mockWindows } from "@tauri-apps/api/mocks";
import type { Block, DayCount, Inline, LoadedSnippet, Marks, Milestone, Settings, Snippet, StatsView } from "./types";
import { strftime, shiftedToday } from "./strftime";

const german = new URLSearchParams(location.search).get("lang") !== "en";

const text = (t: string, marks?: Marks): Inline => (marks ? { type: "text", text: t, marks } : { type: "text", text: t });

function snippet(id: number, trigger: string, label: string, blocks: Block[], extra: Partial<Snippet> = {}): Snippet {
  return {
    id: `019a0f3c-5d2e-7a10-8b3c-0a1b2c3d4e${String(id).padStart(2, "0")}`,
    trigger,
    label,
    content: { blocks },
    enabled: true,
    ...extra,
  };
}

/** The e-mail signature, which the "thanks" snippet includes. */
const SIGNATURE = "019a0f3c-5d2e-7a10-8b3c-0a1b2c3d4e03";

const de: LoadedSnippet[] = [
  { category: [], path: "", snippet: snippet(1, ":mfg", "Grußformel", [{ type: "paragraph", content: [text("Mit freundlichen Grüßen")] }]) },
  { category: [], path: "", snippet: snippet(2, ":datum", "Heutiges Datum", [{ type: "paragraph", content: [{ type: "date_time", format: "%d.%m.%Y" }] }]) },
  {
    category: ["E-Mail"],
    path: "",
    snippet: snippet(3, ":sig", "E-Mail-Signatur", [
      { type: "paragraph", content: [text("Viele Grüße")] },
      { type: "paragraph", content: [text("Max Mustermann", { bold: true }), { type: "line_break" }, text("Musterfirma GmbH · "), text("www.example.org", { link: "https://www.example.org" })] },
    ]),
  },
  {
    category: ["E-Mail"],
    path: "",
    snippet: snippet(
      4,
      ":rr",
      "Rückruf-Notiz",
      [
        { type: "paragraph", content: [text("Rückruf", { bold: true }), text(" vom "), { type: "date_time", format: "%d.%m.%Y, %H:%M" }] },
        {
          type: "bullet_list",
          items: [
            [text("Anrufer: "), { type: "field", name: "anrufer" }],
            [text("Anliegen: "), { type: "field", name: "anliegen" }],
            [text("Rückruf erwünscht: "), { type: "field", name: "rueckruf" }],
          ],
        },
        { type: "paragraph", content: [text("Notiz: "), { type: "cursor" }] },
      ],
      {
        fields: [
          { name: "anrufer", label: "Wer hat angerufen?", kind: { type: "text" } },
          { name: "anliegen", label: "Anliegen", kind: { type: "choice", options: ["Angebot", "Reklamation", "Termin"] }, default: "Angebot" },
          { name: "rueckruf", label: "Rückruf erwünscht?", kind: { type: "yes_no" }, default: "Ja" },
        ],
      },
    ),
  },
  { category: ["Kunden"], path: "", snippet: snippet(5, ":adr", "Firmenadresse", [{ type: "paragraph", content: [text("Musterfirma GmbH")] }, { type: "paragraph", content: [text("Musterstraße 1")] }, { type: "paragraph", content: [text("12345 Musterstadt")] }]) },
  {
    category: ["Kunden"],
    path: "",
    snippet: snippet(6, ":danke", "Dank für Anfrage", [
      { type: "paragraph", content: [text("Vielen Dank für Ihre Anfrage. Wir melden uns innerhalb von zwei Werktagen.")] },
      { type: "paragraph", content: [] },
      { type: "paragraph", content: [{ type: "snippet", id: SIGNATURE }] },
    ]),
  },
];

const en: LoadedSnippet[] = [
  { category: [], path: "", snippet: snippet(1, ":br", "Closing", [{ type: "paragraph", content: [text("Best regards")] }]) },
  { category: [], path: "", snippet: snippet(2, ":date", "Today's date", [{ type: "paragraph", content: [{ type: "date_time", format: "%B %-d, %Y" }] }]) },
  {
    category: ["Email"],
    path: "",
    snippet: snippet(3, ":sig", "Email signature", [
      { type: "paragraph", content: [text("Kind regards")] },
      { type: "paragraph", content: [text("Jane Doe", { bold: true }), { type: "line_break" }, text("Example Ltd · "), text("www.example.org", { link: "https://www.example.org" })] },
    ]),
  },
  {
    category: ["Email"],
    path: "",
    snippet: snippet(
      4,
      ":call",
      "Call note",
      [
        { type: "paragraph", content: [text("Call", { bold: true }), text(" on "), { type: "date_time", format: "%B %-d, %Y, %H:%M" }] },
        {
          type: "bullet_list",
          items: [
            [text("Caller: "), { type: "field", name: "caller" }],
            [text("Topic: "), { type: "field", name: "topic" }],
            [text("Call back: "), { type: "field", name: "callback" }],
          ],
        },
        { type: "paragraph", content: [text("Note: "), { type: "cursor" }] },
      ],
      {
        fields: [
          { name: "caller", label: "Who called?", kind: { type: "text" } },
          { name: "topic", label: "Topic", kind: { type: "choice", options: ["Quote", "Complaint", "Appointment"] }, default: "Quote" },
          { name: "callback", label: "Call back?", kind: { type: "yes_no" }, default: "Yes" },
        ],
      },
    ),
  },
  { category: ["Customers"], path: "", snippet: snippet(5, ":addr", "Company address", [{ type: "paragraph", content: [text("Example Ltd")] }, { type: "paragraph", content: [text("1 Example Street")] }, { type: "paragraph", content: [text("Springfield")] }]) },
  {
    category: ["Customers"],
    path: "",
    snippet: snippet(6, ":thx", "Thanks for the request", [
      { type: "paragraph", content: [text("Thank you for your request. We will get back to you within two working days.")] },
      { type: "paragraph", content: [] },
      { type: "paragraph", content: [{ type: "snippet", id: SIGNATURE }] },
    ]),
  },
];

const library = german ? de : en;

const settings: Settings = {
  version: 1,
  language: german ? "de" : "en",
  paused: false,
  start_with_system: true,
  first_run_complete: true,
  notifications: true,
  search_hotkey: "Ctrl+Shift+Space",
  selection_hotkey: "Ctrl+Alt+S",
  app_rules: [{ app: "game.exe", enabled: false }],
  check_for_updates: true,
  statistics: true,
};

const escape = (s: string) => s.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;");

/** A small stand-in for the Rust renderer. */
function renderHtml(s: Snippet, depth = 0): { html: string; plain: string } {
  const included = (id: string) => {
    const found = library.find((l) => l.snippet.id === id)?.snippet;
    return found && depth < 5 ? renderHtml(found, depth + 1).html : "…";
  };
  const inline = (i: Inline): string => {
    switch (i.type) {
      case "text": {
        let out = escape(i.text);
        if (i.marks?.bold) out = `<strong>${out}</strong>`;
        if (i.marks?.italic) out = `<em>${out}</em>`;
        if (i.marks?.link) out = `<a href="${escape(i.marks.link)}">${out}</a>`;
        return out;
      }
      case "line_break":
        return "<br>";
      case "date_time":
        return escape(strftime(i.format, shiftedToday(i.offset_days ?? 0)));
      case "field": {
        const field = s.fields?.find((f) => f.name === i.name);
        return escape(field?.default ?? `‹${field?.label ?? i.name}›`);
      }
      case "clipboard":
        return "📋";
      case "cursor":
        return "";
      case "snippet":
        // Inside text, the included snippet's paragraphs become lines.
        return included(i.id).replace(/<\/p><p>/g, "<br>").replace(/<\/?p>/g, "");
    }
  };
  const run = (inlines: Inline[] | undefined) => (inlines ?? []).map(inline).join("");
  const html = s.content.blocks
    .map((b) =>
      b.type === "paragraph"
        ? b.content?.length === 1 && b.content[0]?.type === "snippet"
          ? included(b.content[0].id)
          : `<p>${run(b.content) || "<br>"}</p>`
        : `<${b.type === "bullet_list" ? "ul" : "ol"}>${b.items.map((item) => `<li>${run(item)}</li>`).join("")}</${b.type === "bullet_list" ? "ul" : "ol"}>`,
    )
    .join("");
  return { html, plain: html.replace(/<[^>]+>/g, " ").trim() };
}

/** Plausible statistics for the screenshots: busy weekdays, quiet weekends. */
function sampleStats(): StatsView {
  const today = new Date();
  const iso = (d: Date) =>
    `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, "0")}-${String(d.getDate()).padStart(2, "0")}`;
  const days: DayCount[] = [];
  for (let ago = 181; ago >= 0; ago--) {
    const day = new Date(today.getFullYear(), today.getMonth(), today.getDate() - ago);
    const weekend = day.getDay() === 0 || day.getDay() === 6;
    const started = ago < 150;
    // A pseudo-random but fixed pattern; the last ten days form a streak.
    const base = ((ago * 7919) % 13) % 9;
    const count = !started ? 0 : ago < 10 ? Math.max(1, base) : weekend ? (ago % 4 === 0 ? 1 : 0) : base;
    days.push({ date: iso(day), count });
  }
  const insertions = days.reduce((sum, d) => sum + d.count, 0);
  const characters = insertions * 96;
  const minutes = Math.floor(characters / 200);
  const streak = 10;
  const best = 23;
  const ids = (n: number) => `019a0f3c-5d2e-7a10-8b3c-0a1b2c3d4e${String(n).padStart(2, "0")}`;
  const top = [3, 1, 6, 4, 5].map((n, i) => {
    const s = library.find((l) => l.snippet.id === ids(n))?.snippet;
    return { id: ids(n), trigger: s?.trigger ?? null, label: s?.label ?? null, count: [212, 180, 97, 64, 31][i] ?? 0, last_used: iso(today) };
  });
  const reached = (m: Milestone) =>
    m.kind === "insertions" ? insertions >= m.value : m.kind === "hours_saved" ? minutes >= m.value * 60 : best >= m.value;
  const milestones: Milestone[] = [
    ...[10, 50, 100, 500, 1000, 5000, 10000].map((value) => ({ kind: "insertions" as const, value })),
    ...[1, 5, 10, 50, 100].map((value) => ({ kind: "hours_saved" as const, value })),
    ...[3, 7, 30, 100, 365].map((value) => ({ kind: "streak" as const, value })),
  ];
  return {
    enabled: true,
    today: iso(today),
    since: days.find((d) => d.count > 0)?.date ?? null,
    insertions,
    characters_saved: characters,
    minutes_saved: minutes,
    streak,
    best_streak: best,
    days,
    top,
    milestones: milestones.map((milestone) => ({ milestone, reached: reached(milestone) })),
  };
}

mockWindows("main");
mockIPC(
  (cmd, payload) => {
    const args = (payload ?? {}) as Record<string, unknown>;
    switch (cmd) {
      case "app_info":
        return {
          version: "0.1.0",
          library: german ? "C:\\Users\\Max\\AppData\\Roaming\\Typlet\\data\\snippets" : "C:\\Users\\Jane\\AppData\\Roaming\\Typlet\\data\\snippets",
          settings_file: "",
          default_espanso_dir: null,
          open_snippet: new URLSearchParams(location.search).get("open"),
          open_settings: new URLSearchParams(location.search).has("settings"),
          open_update: new URLSearchParams(location.search).has("update"),
          open_stats: new URLSearchParams(location.search).has("stats"),
          system_german: german,
          background_running: true,
        };
      case "load_library":
        return { root: "", snippets: library, errors: [], conflicts: [] };
      case "load_settings":
        return settings;
      case "validate_snippet":
        return null;
      case "preview": {
        const s = args["snippet"] as Snippet;
        const { html, plain } = renderHtml(s);
        const multi = s.content.blocks.length > 1 || s.content.blocks.some((b) => b.type !== "paragraph");
        const formatted = html.includes("<strong>") || html.includes("<a ") || html.includes("<ul>");
        return { html, plain, formatted, method: multi || formatted ? "paste" : "type" };
      }
      case "parse_hotkey":
        return args["text"];
      case "update_status":
      case "check_for_update":
        return {
          current: "0.1.0",
          // Only the update screenshot shows an available update.
          available: new URLSearchParams(location.search).has("update")
            ? { version: "0.2.0", notes: german ? "Schneller und mit Statistik." : "Faster, with statistics.", date: "2026-11-02T10:00:00Z" }
            : null,
          checked_at: "2026-11-02 10:15",
          can_install: true,
        };
      case "load_stats":
        return sampleStats();
      case "plugin:dialog|ask":
        return true;
      default:
        return null;
    }
  },
  { shouldMockEvents: true },
);
