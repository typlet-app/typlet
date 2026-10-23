// SPDX-License-Identifier: GPL-3.0-or-later
// Choosing another snippet to insert into the one being edited.

import { useMemo, useState } from "react";
import { useT } from "../i18n";
import { matchesSearch, searchWords } from "../text";
import type { Snippet } from "../types";

/** How many matches are listed; typing narrows them down. */
const SHOWN = 8;

interface Props {
  /** The snippets that can be inserted (no loops). */
  candidates: Snippet[];
  onPick: (id: string) => void;
  onClose: () => void;
}

export function SnippetPicker({ candidates, onPick, onClose }: Props) {
  const t = useT();
  const [query, setQuery] = useState("");
  const matches = useMemo(() => {
    const words = searchWords(query);
    return candidates.filter((s) => matchesSearch(s, words));
  }, [candidates, query]);
  const first = matches[0];

  return (
    <div className="inspector snippet-picker" role="group" aria-label={t("insert_snippet_title")}>
      {candidates.length === 0 ? (
        <span className="hint grow">{t("snippet_picker_none")}</span>
      ) : (
        <>
          <label className="grow">
            {t("snippet_picker_search")}
            <input
              autoFocus
              type="search"
              value={query}
              placeholder={t("search_placeholder")}
              onChange={(e) => setQuery(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === "Enter") {
                  e.preventDefault();
                  if (first) onPick(first.id);
                } else if (e.key === "Escape") {
                  onClose();
                }
              }}
            />
          </label>
          <ul className="picker-list" aria-live="polite">
            {matches.slice(0, SHOWN).map((s) => (
              <li key={s.id}>
                <button type="button" className="picker-item" onClick={() => onPick(s.id)}>
                  <span>🧩 {s.label || s.trigger}</span>
                  <span className="mono small picker-trigger">{s.trigger}</span>
                </button>
              </li>
            ))}
            {matches.length === 0 && <li className="hint">{t("snippet_picker_empty")}</li>}
            {matches.length > SHOWN && <li className="hint">{t("snippet_picker_more", { count: matches.length - SHOWN })}</li>}
          </ul>
        </>
      )}
      <button type="button" className="link-button" onClick={onClose}>
        {t("cancel")}
      </button>
    </div>
  );
}
