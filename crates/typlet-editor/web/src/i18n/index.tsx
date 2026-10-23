// SPDX-License-Identifier: GPL-3.0-or-later
// Interface texts. English is the source; German follows the glossary in
// docs/design.md. `scripts/check-translations.py` checks both have the same keys.

import { createContext, useContext } from "react";
import de from "./de.json";
import en from "./en.json";

export type Lang = "de" | "en";
export type Key = keyof typeof en;

const TABLES: Record<Lang, Record<string, string>> = { de, en };

/** Replaces `{name}` with values. */
function fill(text: string, vars?: Record<string, string | number>): string {
  if (!vars) return text;
  return text.replace(/\{(\w+)\}/g, (match, name: string) =>
    name in vars ? String(vars[name]) : match,
  );
}

export function translate(lang: Lang, key: Key, vars?: Record<string, string | number>): string {
  return fill(TABLES[lang][key] ?? en[key] ?? key, vars);
}

export const LangContext = createContext<Lang>("en");

/** `const t = useT(); t("save")` */
export function useT() {
  const lang = useContext(LangContext);
  return (key: Key, vars?: Record<string, string | number>) => translate(lang, key, vars);
}

export function useLang(): Lang {
  return useContext(LangContext);
}
