// SPDX-License-Identifier: GPL-3.0-or-later
// Importing snippets from espanso.

import { open } from "@tauri-apps/plugin-dialog";
import { useState } from "react";
import { api, errorText } from "../api";
import { useT, type Key } from "../i18n";
import type { ImportSummary, SkipReason } from "../types";

interface Props {
  defaultDir: string | null;
  onImported: () => void;
}

export function ImportPage({ defaultDir, onImported }: Props) {
  const t = useT();
  const [dir, setDir] = useState(defaultDir ?? "");
  const [busy, setBusy] = useState(false);
  const [summary, setSummary] = useState<ImportSummary | null>(null);
  const [error, setError] = useState<string | null>(null);

  const choose = async () => {
    const chosen = await open({ directory: true, title: t("import_choose") });
    if (typeof chosen === "string") setDir(chosen);
  };

  const run = () => {
    setBusy(true);
    setError(null);
    api
      .importEspanso(dir)
      .then((result) => {
        setSummary(result);
        onImported();
      })
      .catch((e: unknown) => setError(errorText(e)))
      .finally(() => setBusy(false));
  };

  const reason = (r: SkipReason) => {
    const text = t(`skip_${r.kind}` as Key);
    return "detail" in r ? `${text}: ${r.detail}` : text;
  };

  return (
    <div className="page import-page">
      <header className="page-header">
        <h1>{t("import_title")}</h1>
      </header>
      <section className="panel">
        <p>{t("import_intro")}</p>
        <div className="row">
          <input className="mono grow" value={dir} onChange={(e) => setDir(e.target.value)} placeholder={t("import_folder_placeholder")} />
          <button type="button" className="secondary" onClick={() => void choose()}>
            {t("import_choose")}
          </button>
          <button type="button" className="primary" disabled={busy || dir.trim() === ""} onClick={run}>
            {busy ? t("importing") : t("import_start")}
          </button>
        </div>
        <p className="hint">{defaultDir ? t("import_found", { dir: defaultDir }) : t("import_not_found")}</p>
        {error && <p className="error-banner">{error}</p>}
      </section>

      {summary && (
        <section className="panel">
          <h3>{t("import_result", { saved: summary.saved, files: summary.files })}</h3>
          {summary.imported.length > 0 && (
            <ul className="import-list">
              {summary.imported.map((item) => (
                <li key={item.trigger}>
                  <span className="chip-trigger">{item.trigger}</span> {item.label}
                  {item.warnings.map((w) => (
                    <span key={w} className="badge warning">
                      {t(`warn_${w}` as Key)}
                    </span>
                  ))}
                </li>
              ))}
            </ul>
          )}
          {summary.existing.length > 0 && (
            <p className="hint">{t("import_existing", { triggers: summary.existing.join(", ") })}</p>
          )}
          {summary.skipped.length > 0 && (
            <>
              <h4>{t("import_skipped")}</h4>
              <ul className="import-list skipped">
                {summary.skipped.map((item, i) => (
                  <li key={`${item.source}-${item.trigger}-${i}`}>
                    {item.trigger && <span className="chip-trigger">{item.trigger}</span>} {reason(item.reason)}
                    <span className="hint"> · {item.source}</span>
                  </li>
                ))}
              </ul>
            </>
          )}
        </section>
      )}
    </div>
  );
}
