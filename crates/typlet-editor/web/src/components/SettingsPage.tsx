// SPDX-License-Identifier: GPL-3.0-or-later
// All settings: language, start-up, shortcuts, snippet folder, app rules.

import { open } from "@tauri-apps/plugin-dialog";
import { useEffect, useState } from "react";
import { api, errorText } from "../api";
import { useT } from "../i18n";
import type { AppRule, InsertMode, NewlineMode, Settings } from "../types";
import { HotkeyInput } from "./HotkeyInput";

interface Props {
  settings: Settings;
  libraryPath: string;
  onSaved: (settings: Settings) => void;
  onCheckUpdates: () => void;
}

const BUILT_IN = [
  "excel.exe",
  "ms-teams.exe",
  "whatsapp.exe",
  "slack.exe",
  "discord.exe",
  "chrome.exe",
  "msedge.exe",
  "firefox.exe",
  "mstsc.exe",
];

export function SettingsPage({ settings: initial, libraryPath, onSaved, onCheckUpdates }: Props) {
  const t = useT();
  const [settings, setSettings] = useState(initial);
  const [status, setStatus] = useState<{ kind: "saved" | "error"; text: string } | null>(null);
  useEffect(() => setSettings(initial), [initial]);

  const dirty = JSON.stringify(settings) !== JSON.stringify(initial);
  const set = (changes: Partial<Settings>) => {
    setSettings({ ...settings, ...changes });
    setStatus(null);
  };
  const setRule = (index: number, rule: AppRule) =>
    set({ app_rules: settings.app_rules.map((r, i) => (i === index ? rule : r)) });

  const save = () => {
    const cleaned: Settings = {
      ...settings,
      app_rules: settings.app_rules.filter((rule) => rule.app.trim() !== "").map((rule) => ({ ...rule, app: rule.app.trim() })),
    };
    api
      .saveSettings(cleaned)
      .then(() => {
        onSaved(cleaned);
        setStatus({ kind: "saved", text: t("settings_saved") });
      })
      .catch((error: unknown) => setStatus({ kind: "error", text: errorText(error) }));
  };

  const chooseFolder = async () => {
    const dir = await open({ directory: true, title: t("library_choose") });
    if (typeof dir === "string") set({ library_path: dir });
  };

  return (
    <div className="page settings-page">
      <header className="page-header">
        <h1>{t("settings")}</h1>
        <button type="button" className="primary" disabled={!dirty} onClick={save}>
          {dirty ? t("save") : t("saved")}
        </button>
      </header>
      {status && <p className={status.kind === "error" ? "error-banner" : "success-banner"}>{status.text}</p>}

      <section className="panel">
        <h3>{t("settings_general")}</h3>
        <label className="row">
          <span>{t("language")}</span>
          <select value={settings.language} onChange={(e) => set({ language: e.target.value as Settings["language"] })}>
            <option value="system">{t("language_system")}</option>
            <option value="de">Deutsch</option>
            <option value="en">English</option>
          </select>
        </label>
        <label className="row switch">
          <input type="checkbox" checked={settings.start_with_system} onChange={(e) => set({ start_with_system: e.target.checked })} />
          <span>{t("start_with_system")}</span>
        </label>
        <label className="row switch">
          <input type="checkbox" checked={settings.notifications} onChange={(e) => set({ notifications: e.target.checked })} />
          <span>{t("notifications")}</span>
        </label>
        <label className="row switch">
          <input type="checkbox" checked={settings.paused} onChange={(e) => set({ paused: e.target.checked })} />
          <span>{t("paused")}</span>
        </label>
      </section>

      <section className="panel">
        <h3>{t("updates")}</h3>
        <label className="row switch">
          <input type="checkbox" checked={settings.check_for_updates} onChange={(e) => set({ check_for_updates: e.target.checked })} />
          <span>{t("check_for_updates")}</span>
        </label>
        <p className="muted small">{t("check_for_updates_hint")}</p>
        <div className="row-buttons">
          <button type="button" className="secondary" onClick={onCheckUpdates}>
            {t("check_now")}
          </button>
        </div>
      </section>

      <section className="panel">
        <h3>{t("shortcuts")}</h3>
        <div className="row">
          <span>{t("search_hotkey")}</span>
          <HotkeyInput value={settings.search_hotkey} onChange={(search_hotkey) => set({ search_hotkey })} />
        </div>
        <div className="row">
          <span>{t("selection_hotkey")}</span>
          <HotkeyInput value={settings.selection_hotkey} onChange={(selection_hotkey) => set({ selection_hotkey })} />
        </div>
      </section>

      <section className="panel">
        <h3>{t("library_folder")}</h3>
        <p className="mono path">{settings.library_path || libraryPath}</p>
        <div className="row-buttons">
          <button type="button" className="secondary" onClick={() => void chooseFolder()}>
            {t("library_choose")}
          </button>
          {settings.library_path && (
            <button type="button" className="secondary" onClick={() => set({ library_path: null })}>
              {t("library_default")}
            </button>
          )}
          <button type="button" className="link-button" onClick={() => void api.openFolder("library")}>
            {t("open_folder")}
          </button>
        </div>
        <p className="hint">{t("library_help")}</p>
      </section>

      <section className="panel">
        <h3>{t("app_rules")}</h3>
        <p className="hint">{t("app_rules_help", { apps: BUILT_IN.join(", ") })}</p>
        {settings.app_rules.length > 0 && (
          <table className="rules">
            <thead>
              <tr>
                <th>{t("rule_app")}</th>
                <th>{t("rule_enabled")}</th>
                <th>{t("rule_newline")}</th>
                <th>{t("rule_insert")}</th>
                <th>{t("rule_plain")}</th>
                <th>{t("rule_delay")}</th>
                <th />
              </tr>
            </thead>
            <tbody>
              {settings.app_rules.map((rule, index) => (
                <tr key={index}>
                  <td>
                    <input className="mono" value={rule.app} placeholder="program.exe" onChange={(e) => setRule(index, { ...rule, app: e.target.value })} />
                  </td>
                  <td>
                    <input type="checkbox" checked={rule.enabled !== false} onChange={(e) => setRule(index, { ...rule, enabled: e.target.checked })} />
                  </td>
                  <td>
                    <select value={rule.newline ?? ""} onChange={(e) => setRule(index, { ...rule, newline: (e.target.value || null) as NewlineMode | null })}>
                      <option value="">{t("rule_default")}</option>
                      <option value="enter">{t("newline_enter")}</option>
                      <option value="shift_enter">{t("newline_shift_enter")}</option>
                    </select>
                  </td>
                  <td>
                    <select value={rule.insert_mode ?? ""} onChange={(e) => setRule(index, { ...rule, insert_mode: (e.target.value || null) as InsertMode | null })}>
                      <option value="">{t("rule_default")}</option>
                      <option value="type">{t("insert_mode_type")}</option>
                      <option value="paste">{t("insert_mode_paste")}</option>
                    </select>
                  </td>
                  <td>
                    <input type="checkbox" checked={rule.plain_only === true} onChange={(e) => setRule(index, { ...rule, plain_only: e.target.checked || null })} />
                  </td>
                  <td>
                    <input
                      type="number"
                      min={0}
                      max={10000}
                      step={100}
                      placeholder="300"
                      value={rule.restore_delay_ms ?? ""}
                      onChange={(e) => setRule(index, { ...rule, restore_delay_ms: e.target.value === "" ? null : Number(e.target.value) })}
                    />
                  </td>
                  <td>
                    <button type="button" className="link-button danger" onClick={() => set({ app_rules: settings.app_rules.filter((_, i) => i !== index) })}>
                      {t("remove")}
                    </button>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
        <button type="button" className="secondary" onClick={() => set({ app_rules: [...settings.app_rules, { app: "", enabled: true }] })}>
          + {t("rule_add")}
        </button>
      </section>
    </div>
  );
}
