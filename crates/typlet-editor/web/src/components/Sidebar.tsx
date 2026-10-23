// SPDX-License-Identifier: GPL-3.0-or-later
// Snippet list with search, grouped by category.

import { useMemo, useState } from "react";
import { useT } from "../i18n";
import { normalise, previewText } from "../text";
import type { LoadedSnippet } from "../types";

interface Props {
  snippets: LoadedSnippet[];
  selectedId: string | null;
  page: "snippet" | "settings" | "import" | "home";
  conflicted: Set<string>;
  backgroundRunning: boolean;
  onSelect: (id: string) => void;
  onNew: () => void;
  onSettings: () => void;
  onImport: () => void;
  onStartBackground: () => void;
}

export function Sidebar(props: Props) {
  const t = useT();
  const [query, setQuery] = useState("");

  const groups = useMemo(() => {
    const words = normalise(query).split(/\s+/).filter(Boolean);
    const matches = props.snippets.filter(({ snippet }) => {
      if (words.length === 0) return true;
      const haystack = normalise(`${snippet.trigger} ${snippet.label} ${previewText(snippet.content)}`);
      return words.every((word) => haystack.includes(word));
    });
    const byCategory = new Map<string, LoadedSnippet[]>();
    for (const item of matches) {
      const key = item.category.join(" / ");
      byCategory.set(key, [...(byCategory.get(key) ?? []), item]);
    }
    return [...byCategory.entries()]
      .sort(([a], [b]) => (a === "" ? -1 : b === "" ? 1 : a.localeCompare(b)))
      .map(([category, items]) => ({
        category,
        items: items.sort((a, b) => a.snippet.label.localeCompare(b.snippet.label)),
      }));
  }, [props.snippets, query]);

  return (
    <aside className="sidebar">
      <div className="brand">
        <img src="/logo.svg" alt="" width={28} height={28} />
        <span>Typlet</span>
        <button type="button" className="primary small" onClick={props.onNew}>
          + {t("new_snippet")}
        </button>
      </div>
      <input
        className="search"
        type="search"
        placeholder={t("search_placeholder")}
        value={query}
        onChange={(e) => setQuery(e.target.value)}
        aria-label={t("search_placeholder")}
      />
      <nav className="snippet-list" aria-label={t("snippets")}>
        {groups.length === 0 && (
          <p className="hint empty">{props.snippets.length === 0 ? t("no_snippets") : t("no_matches")}</p>
        )}
        {groups.map((group) => (
          <section key={group.category}>
            <h4>{group.category === "" ? t("category_none") : group.category}</h4>
            <ul>
              {group.items.map(({ snippet }) => (
                <li key={snippet.id}>
                  <button
                    type="button"
                    className={`snippet-item${props.selectedId === snippet.id && props.page === "snippet" ? " is-selected" : ""}${snippet.enabled === false ? " is-disabled" : ""}`}
                    onClick={() => props.onSelect(snippet.id)}
                  >
                    <span className="item-label">
                      {props.conflicted.has(snippet.id) && (
                        <span className="warning-dot" title={t("conflict_in_list")}>
                          ⚠
                        </span>
                      )}
                      {snippet.label || snippet.trigger}
                    </span>
                    <span className="chip-trigger">{snippet.trigger}</span>
                  </button>
                </li>
              ))}
            </ul>
          </section>
        ))}
      </nav>
      <footer className="sidebar-footer">
        <button type="button" className={`nav${props.page === "settings" ? " is-selected" : ""}`} onClick={props.onSettings}>
          ⚙ {t("settings")}
        </button>
        <button type="button" className={`nav${props.page === "import" ? " is-selected" : ""}`} onClick={props.onImport}>
          ⇩ {t("import")}
        </button>
        <div className={`status ${props.backgroundRunning ? "ok" : "off"}`}>
          <span className="dot" aria-hidden />
          {props.backgroundRunning ? t("background_running") : t("background_stopped")}
          {!props.backgroundRunning && (
            <button type="button" className="link-button" onClick={props.onStartBackground}>
              {t("start")}
            </button>
          )}
        </div>
      </footer>
    </aside>
  );
}
