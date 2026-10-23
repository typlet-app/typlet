// SPDX-License-Identifier: GPL-3.0-or-later
// Typed wrappers around the editor's Rust commands (src/commands.rs).

import { invoke } from "@tauri-apps/api/core";
import type { AppInfo, ImportSummary, LibraryView, Preview, Settings, Snippet, UpdateView } from "./types";

export const api = {
  appInfo: () => invoke<AppInfo>("app_info"),
  loadLibrary: () => invoke<LibraryView>("load_library"),
  saveSnippet: (snippet: Snippet, category: string[]) =>
    invoke<void>("save_snippet", { snippet, category }),
  deleteSnippet: (id: string) => invoke<void>("delete_snippet", { id }),
  validateSnippet: (snippet: Snippet) => invoke<string | null>("validate_snippet", { snippet }),
  preview: (snippet: Snippet, fields: Record<string, string> = {}) =>
    invoke<Preview>("preview", { snippet, fields }),
  loadSettings: () => invoke<Settings>("load_settings"),
  saveSettings: (settings: Settings) => invoke<void>("save_settings", { settings }),
  parseHotkey: (text: string) => invoke<string>("parse_hotkey", { text }),
  importEspanso: (dir: string) => invoke<ImportSummary>("import_espanso", { dir }),
  openFolder: (which: "library" | "data") => invoke<void>("open_folder", { which }),
  startBackground: () => invoke<boolean>("start_background"),
  /** Opens one of Typlet's web pages in the browser. */
  openLink: (which: "download") => invoke<void>("open_link", { which }),
  updateStatus: () => invoke<UpdateView>("update_status"),
  checkForUpdate: () => invoke<UpdateView>("check_for_update"),
  installUpdate: () => invoke<void>("install_update"),
};

/** Error text from a failed command. */
export function errorText(error: unknown): string {
  return typeof error === "string" ? error : error instanceof Error ? error.message : String(error);
}
