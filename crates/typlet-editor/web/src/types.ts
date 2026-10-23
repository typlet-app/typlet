// SPDX-License-Identifier: GPL-3.0-or-later
// TypeScript mirror of the snippet and settings files.
// Reference: docs/snippet-format.md and docs/settings-format.md.

export interface Marks {
  bold?: boolean;
  italic?: boolean;
  underline?: boolean;
  link?: string;
}

export type Inline =
  | { type: "text"; text: string; marks?: Marks }
  | { type: "line_break" }
  | { type: "date_time"; format: string; offset_days?: number }
  | { type: "cursor" }
  | { type: "clipboard" }
  | { type: "field"; name: string }
  /** Another snippet, inserted here (docs/adr/0016-snippets-inside-snippets.md). */
  | { type: "snippet"; id: string };

export type Block =
  | { type: "paragraph"; content?: Inline[] }
  | { type: "bullet_list"; items: Inline[][] }
  | { type: "ordered_list"; items: Inline[][] };

export interface TypletDocument {
  blocks: Block[];
}

export type FieldKind =
  | { type: "text" }
  | { type: "multiline_text" }
  | { type: "choice"; options: string[] }
  | { type: "yes_no" };

export interface FieldDef {
  name: string;
  label: string;
  kind: FieldKind;
  default?: string;
}

export type InsertMode = "auto" | "type" | "paste";

export interface Snippet {
  version?: number;
  id: string;
  trigger: string;
  label: string;
  content: TypletDocument;
  fields?: FieldDef[];
  enabled?: boolean;
  insert_mode?: InsertMode;
  hotkey?: string | null;
}

/** A snippet as loaded from the library, with its file and category. */
export interface LoadedSnippet {
  snippet: Snippet;
  path: string;
  category: string[];
}

export type Conflict =
  | { kind: "duplicate"; trigger: string; snippets: [string, string] }
  | { kind: "shadowed"; by: string; shadowed: string };

export interface LibraryView {
  root: string;
  snippets: LoadedSnippet[];
  errors: string[];
  conflicts: Conflict[];
}

export type Language = "system" | "de" | "en";
export type NewlineMode = "enter" | "shift_enter";

export interface AppRule {
  app: string;
  enabled?: boolean;
  insert_mode?: InsertMode | null;
  newline?: NewlineMode | null;
  plain_only?: boolean | null;
  restore_delay_ms?: number | null;
}

export interface Settings {
  version: number;
  language: Language;
  paused: boolean;
  start_with_system: boolean;
  first_run_complete: boolean;
  notifications: boolean;
  search_hotkey: string | null;
  selection_hotkey: string | null;
  library_path?: string | null;
  app_rules: AppRule[];
  check_for_updates: boolean;
  /** Count inserted snippets for the statistics (numbers only). */
  statistics: boolean;
}

export interface Preview {
  plain: string;
  html: string;
  method: "type" | "paste";
  formatted: boolean;
}

export type ImportWarning =
  | "word_boundary"
  | "propagate_case"
  | "formatting_simplified"
  | "offset_rounded";

export type SkipReason =
  | { kind: "regex" }
  | { kind: "image" }
  | { kind: "no_text" }
  | { kind: "unsupported_variable"; detail: string }
  | { kind: "invalid_trigger"; detail: string }
  | { kind: "missing_match"; detail: string }
  | { kind: "invalid"; detail: string }
  | { kind: "unreadable"; detail: string };

export interface ImportSummary {
  files: number;
  imported: { trigger: string; label: string; source: string; warnings: ImportWarning[] }[];
  skipped: { source: string; trigger: string; reason: SkipReason }[];
  /** Triggers that already existed and were not imported again. */
  existing: string[];
  saved: number;
}

export interface AppInfo {
  version: string;
  library: string;
  settings_file: string;
  default_espanso_dir: string | null;
  open_snippet: string | null;
  open_settings: boolean;
  open_update: boolean;
  open_stats: boolean;
  system_german: boolean;
  background_running: boolean;
}

/** What is known about new versions (src/updates.rs). */
export interface UpdateView {
  current: string;
  available: { version: string; notes?: string; date?: string } | null;
  checked_at: string | null;
  can_install: boolean;
}

/** Something worth celebrating (typlet_core::stats::Milestone). */
export type Milestone =
  | { kind: "insertions"; value: number }
  | { kind: "hours_saved"; value: number }
  | { kind: "streak"; value: number };

export interface DayCount {
  /** `YYYY-MM-DD` */
  date: string;
  count: number;
}

/** The statistics page (src/stats.rs). Numbers only, never text. */
export interface StatsView {
  enabled: boolean;
  today: string;
  since: string | null;
  insertions: number;
  characters_saved: number;
  minutes_saved: number;
  streak: number;
  best_streak: number;
  days: DayCount[];
  top: { id: string; trigger: string | null; label: string | null; count: number; last_used: string }[];
  milestones: { milestone: Milestone; reached: boolean }[];
}
