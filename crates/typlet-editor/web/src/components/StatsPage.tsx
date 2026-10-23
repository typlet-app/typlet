// SPDX-License-Identifier: GPL-3.0-or-later
// How much typing Typlet saved: totals, streak, activity chart, most used
// snippets and milestones (ADR 0017). Only numbers, never text.

import { ask } from "@tauri-apps/plugin-dialog";
import { useCallback, useEffect, useState } from "react";
import { api, errorText } from "../api";
import { useLang, useT } from "../i18n";
import { activityLevel, dayCount, duration, parseDay, weekColumns } from "../stats";
import type { Milestone, StatsView } from "../types";

interface Props {
  onOpenSnippet: (id: string) => void;
  onSettings: () => void;
}

const MILESTONE_ICON: Record<Milestone["kind"], string> = {
  insertions: "🧩",
  hours_saved: "⏱️",
  streak: "🔥",
};

export function StatsPage({ onOpenSnippet, onSettings }: Props) {
  const t = useT();
  const lang = useLang();
  const [view, setView] = useState<StatsView | null>(null);
  const [error, setError] = useState<string | null>(null);

  const load = useCallback(() => {
    api.loadStats().then(setView, (e: unknown) => setError(errorText(e)));
  }, []);
  useEffect(load, [load]);

  if (error) return <p className="error-banner">{error}</p>;
  if (!view) return null;

  const number = (n: number) => n.toLocaleString(lang);
  const date = (iso: string) => parseDay(iso).toLocaleDateString(lang);
  const days = (count: number) => dayCount(t, count);
  const milestoneText = (m: Milestone) =>
    m.kind === "insertions"
      ? t("milestone_insertions", { count: number(m.value) })
      : m.kind === "hours_saved"
        ? m.value === 1
          ? t("milestone_hour")
          : t("milestone_hours", { count: number(m.value) })
        : t("milestone_streak", { count: number(m.value) });

  const reset = async () => {
    if (!(await ask(t("stats_reset_confirm"), { title: "Typlet", kind: "warning" }))) return;
    api.resetStats().then(load, (e: unknown) => setError(errorText(e)));
  };

  const max = Math.max(1, ...view.days.map((d) => d.count));
  const topMax = Math.max(1, ...view.top.map((s) => s.count));

  return (
    <div className="page stats-page">
      <header className="page-header">
        <h1>📊 {t("stats")}</h1>
      </header>

      {!view.enabled && (
        <p className="info-banner">
          {t("stats_off")}{" "}
          <button type="button" className="link-button" onClick={onSettings}>
            {t("stats_turn_on")}
          </button>
        </p>
      )}

      <div className="kpis">
        <div className="kpi">
          <span className="kpi-value">{number(view.insertions)}</span>
          <span className="kpi-label">🧩 {t("stats_inserted")}</span>
        </div>
        <div className="kpi" title={t("stats_time_hint")}>
          <span className="kpi-value">{duration(t, view.minutes_saved)}</span>
          <span className="kpi-label">⏱️ {t("stats_time_saved")}</span>
        </div>
        <div className="kpi">
          <span className="kpi-value">{number(view.characters_saved)}</span>
          <span className="kpi-label">⌨️ {t("stats_characters_saved")}</span>
        </div>
        <div className="kpi">
          <span className="kpi-value">{days(view.streak)}</span>
          <span className="kpi-label">
            🔥 {t("stats_streak")} · {t("stats_best", { days: days(view.best_streak) })}
          </span>
        </div>
      </div>

      {view.insertions === 0 && view.enabled && <p className="panel hint">{t("stats_empty")}</p>}

      <section className="panel">
        <h3>{t("stats_activity")}</h3>
        <div className="heatmap" role="img" aria-label={t("stats_activity")}>
          {weekColumns(view.days).map((week) => (
            <div className="heatmap-week" key={week[0]?.date ?? "start"}>
              {week.map((day, index) =>
                day ? (
                  <span
                    key={day.date}
                    className={`heat heat-${activityLevel(day.count, max)}`}
                    title={t("stats_day_title", { date: date(day.date), count: number(day.count) })}
                  />
                ) : (
                  <span key={`pad-${index}`} className="heat heat-pad" />
                ),
              )}
            </div>
          ))}
        </div>
        <div className="heat-legend small">
          {t("stats_less")}
          {[0, 1, 2, 3, 4].map((level) => (
            <span key={level} className={`heat heat-${level}`} />
          ))}
          {t("stats_more")}
        </div>
      </section>

      {view.top.length > 0 && (
        <section className="panel">
          <h3>{t("stats_top")}</h3>
          <ol className="top-list">
            {view.top.map((item) => (
              <li key={item.id}>
                {item.trigger ? (
                  <button type="button" className="link-button top-name" onClick={() => onOpenSnippet(item.id)}>
                    {item.label || item.trigger}
                  </button>
                ) : (
                  <span className="top-name muted">{t("stats_deleted")}</span>
                )}
                {item.trigger && <span className="chip-trigger">{item.trigger}</span>}
                <span className="top-bar" aria-hidden>
                  <span style={{ width: `${(100 * item.count) / topMax}%` }} />
                </span>
                <span className="top-count">{t("stats_times", { count: number(item.count) })}</span>
              </li>
            ))}
          </ol>
        </section>
      )}

      <section className="panel">
        <h3>{t("stats_milestones")}</h3>
        <ul className="badges">
          {view.milestones.map(({ milestone, reached }) => (
            <li
              key={`${milestone.kind}-${milestone.value}`}
              className={`badge-item${reached ? " is-reached" : ""}`}
              title={reached ? t("milestone_reached") : t("milestone_open")}
            >
              <span className="badge-icon" aria-hidden>
                {reached ? MILESTONE_ICON[milestone.kind] : "🔒"}
              </span>
              <span>{milestoneText(milestone)}</span>
            </li>
          ))}
        </ul>
      </section>

      <p className="muted small">
        {view.since ? t("stats_privacy_since", { since: date(view.since) }) : t("stats_privacy")}{" "}
        {view.insertions > 0 && (
          <button type="button" className="link-button" onClick={() => void reset()}>
            {t("stats_reset")}
          </button>
        )}
      </p>
    </div>
  );
}
