// SPDX-License-Identifier: GPL-3.0-or-later
// Looking for and installing a new version (ADR 0015). Opened from the
// settings, the home page banner or the tray ("Install update").

import { listen } from "@tauri-apps/api/event";
import { useCallback, useEffect, useState } from "react";
import { api, errorText } from "../api";
import { useLang, useT } from "../i18n";
import type { UpdateView } from "../types";

interface Progress {
  downloaded: number;
  total: number | null;
}

type Phase = "idle" | "checking" | "downloading" | "restarting";

export function UpdatePage({ onStatus }: { onStatus: (view: UpdateView) => void }) {
  const t = useT();
  const lang = useLang();
  const [view, setView] = useState<UpdateView | null>(null);
  const [phase, setPhase] = useState<Phase>("checking");
  const [progress, setProgress] = useState<Progress | null>(null);
  const [error, setError] = useState<string | null>(null);

  const check = useCallback(async () => {
    setPhase("checking");
    setError(null);
    try {
      const next = await api.checkForUpdate();
      setView(next);
      onStatus(next);
    } catch (e) {
      setError(errorText(e));
      setView(await api.updateStatus());
    } finally {
      setPhase("idle");
    }
  }, [onStatus]);

  // Opening the page always asks for the newest state.
  useEffect(() => {
    void check();
  }, [check]);

  useEffect(() => {
    const off = listen<Progress>("update-progress", (event) => setProgress(event.payload));
    return () => {
      void off.then((unlisten) => unlisten());
    };
  }, []);

  const install = async () => {
    setPhase("downloading");
    setError(null);
    setProgress(null);
    try {
      // On Windows the installer takes over and this window closes; on
      // Linux Typlet restarts when the call returns.
      await api.installUpdate();
      setPhase("restarting");
    } catch (e) {
      setError(errorText(e));
      setPhase("idle");
    }
  };

  const available = view?.available ?? null;
  const percent =
    progress?.total && progress.total > 0 ? Math.min(100, Math.round((progress.downloaded / progress.total) * 100)) : null;

  return (
    <div className="page update-page">
      <header className="page-header">
        <h1>{t("updates")}</h1>
      </header>
      {view && <p className="muted">{t("update_current", { version: view.current })}</p>}
      {error && <p className="error-banner">{t("update_failed", { problem: error })}</p>}

      {phase === "checking" && <p>{t("update_checking")}</p>}

      {phase !== "checking" && available && (
        <section className="panel">
          <h2>{t("update_available", { version: available.version })}</h2>
          {available.date && <p className="muted small">{t("update_published", { date: formatDate(available.date, lang) })}</p>}
          {available.notes && <p className="release-notes">{available.notes}</p>}
          {view?.can_install ? (
            <>
              <div className="row-buttons">
                <button type="button" className="primary" disabled={phase !== "idle"} onClick={() => void install()}>
                  {t("update_install")}
                </button>
              </div>
              {phase === "downloading" && (
                <div className="progress" role="progressbar" aria-valuenow={percent ?? undefined} aria-valuemin={0} aria-valuemax={100}>
                  <div className="progress-bar" style={{ width: `${percent ?? 15}%` }} />
                </div>
              )}
              <p className="muted small">
                {phase === "downloading" ? t("update_downloading") : phase === "restarting" ? t("update_restarting") : t("update_restart_note")}
              </p>
            </>
          ) : (
            <>
              <p>{t("update_manual")}</p>
              <div className="row-buttons">
                <button type="button" className="primary" onClick={() => void api.openLink("download")}>
                  {t("update_download_page")}
                </button>
              </div>
            </>
          )}
        </section>
      )}

      {phase === "idle" && view && !available && !error && (
        <section className="panel">
          <p className="success-banner">{t("update_up_to_date")}</p>
          {view.checked_at && <p className="muted small">{t("update_last_checked", { time: view.checked_at })}</p>}
          <div className="row-buttons">
            <button type="button" className="secondary" onClick={() => void check()}>
              {t("update_check_again")}
            </button>
          </div>
        </section>
      )}
    </div>
  );
}

/** `2026-11-02T10:00:00Z` → `02.11.2026` (German) or `2 Nov 2026` (English). */
function formatDate(date: string, lang: "de" | "en"): string {
  const parsed = new Date(date);
  if (Number.isNaN(parsed.getTime())) return date;
  return parsed.toLocaleDateString(lang === "de" ? "de-DE" : "en-GB", { day: lang === "de" ? "2-digit" : "numeric", month: lang === "de" ? "2-digit" : "short", year: "numeric" });
}
