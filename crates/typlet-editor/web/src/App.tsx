// SPDX-License-Identifier: GPL-3.0-or-later
// The editor window: sidebar with all snippets, and the snippet editor,
// settings or import page.

import { listen } from "@tauri-apps/api/event";
import { ask } from "@tauri-apps/plugin-dialog";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { api, errorText } from "./api";
import { ImportPage } from "./components/ImportPage";
import { SettingsPage } from "./components/SettingsPage";
import { Sidebar } from "./components/Sidebar";
import { UpdatePage } from "./components/UpdatePage";
import { normalised, SnippetEditor, type Draft } from "./editor/SnippetEditor";
import { LangContext, translate, type Key, type Lang } from "./i18n";
import type { AppInfo, LibraryView, Settings, UpdateView } from "./types";
import { uuidv7 } from "./uuid";

type Page = "home" | "snippet" | "settings" | "import" | "update";

const DEFAULT_SETTINGS: Settings = {
  version: 1,
  language: "system",
  paused: false,
  start_with_system: true,
  first_run_complete: false,
  notifications: true,
  search_hotkey: "Ctrl+Shift+Space",
  selection_hotkey: "Ctrl+Alt+S",
  app_rules: [],
  check_for_updates: true,
};

const snapshot = (draft: Draft | null) => (draft ? JSON.stringify([normalised(draft.snippet), draft.category]) : "");

export function App() {
  const [info, setInfo] = useState<AppInfo | null>(null);
  const [settings, setSettings] = useState<Settings>(DEFAULT_SETTINGS);
  const [settingsError, setSettingsError] = useState<string | null>(null);
  const [library, setLibrary] = useState<LibraryView | null>(null);
  const [page, setPage] = useState<Page>("home");
  const [draft, setDraft] = useState<Draft | null>(null);
  const [saved, setSaved] = useState("");
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [update, setUpdate] = useState<UpdateView | null>(null);

  const lang: Lang =
    settings.language === "system" ? (info?.system_german ? "de" : "en") : settings.language;
  const t = useCallback((key: Key, vars?: Record<string, string | number>) => translate(lang, key, vars), [lang]);
  const dirty = draft !== null && snapshot(draft) !== saved;
  const dirtyRef = useRef(dirty);
  dirtyRef.current = dirty;

  const reload = useCallback(async () => {
    setLibrary(await api.loadLibrary());
  }, []);

  const refreshInfo = useCallback(async () => setInfo(await api.appInfo()), []);

  /** Asks before throwing away unsaved changes. */
  const mayLeave = useCallback(async () => {
    if (!dirtyRef.current) return true;
    return ask(translate(lang, "discard_changes"), { title: "Typlet", kind: "warning" });
  }, [lang]);

  const openSnippet = useCallback(
    async (id: string, view: LibraryView | null = library) => {
      if (!(await mayLeave())) return;
      const found = view?.snippets.find((s) => s.snippet.id === id);
      if (!found) return;
      const next: Draft = { snippet: structuredClone(found.snippet), category: [...found.category] };
      setDraft(next);
      setSaved(snapshot(next));
      setError(null);
      setPage("snippet");
    },
    [library, mayLeave],
  );

  // Start-up: settings, library, and what the command line asked for.
  useEffect(() => {
    void (async () => {
      const appInfo = await api.appInfo();
      setInfo(appInfo);
      try {
        setSettings(await api.loadSettings());
      } catch (e) {
        setSettingsError(errorText(e));
      }
      const view = await api.loadLibrary();
      setLibrary(view);
      // What the last check found; no network here.
      api.updateStatus().then(setUpdate, () => undefined);
      if (appInfo.open_update) {
        setPage("update");
      } else if (appInfo.open_snippet) {
        const found = view.snippets.find((s) => s.snippet.id === appInfo.open_snippet);
        if (found) {
          const next: Draft = { snippet: structuredClone(found.snippet), category: [...found.category] };
          setDraft(next);
          setSaved(snapshot(next));
          setPage("snippet");
        }
      } else if (appInfo.open_settings) {
        setPage("settings");
      }
    })();
  }, []);

  // Requests from a second start (for example "Edit in the editor").
  useEffect(() => {
    const offSnippet = listen<string>("open-snippet", (event) => {
      void api.loadLibrary().then((view) => {
        setLibrary(view);
        void openSnippet(event.payload, view);
      });
    });
    const offSettings = listen("open-settings", () => setPage("settings"));
    const offUpdate = listen("open-update", () => setPage("update"));
    return () => {
      void offSnippet.then((off) => off());
      void offSettings.then((off) => off());
      void offUpdate.then((off) => off());
    };
  }, [openSnippet]);

  const pageRef = useRef(page);
  pageRef.current = page;

  // Pick up changes made elsewhere: snippets created from a selection,
  // pausing in the tray menu. Settings being edited are left alone.
  useEffect(() => {
    const onFocus = () => {
      void reload();
      void refreshInfo();
      if (pageRef.current !== "settings") {
        api.loadSettings().then(setSettings, () => undefined);
      }
    };
    window.addEventListener("focus", onFocus);
    return () => window.removeEventListener("focus", onFocus);
  }, [reload, refreshInfo]);

  const save = useCallback(async () => {
    if (!draft) return;
    const snippet = normalised(draft.snippet);
    setSaving(true);
    setError(null);
    try {
      const problem = await api.validateSnippet(snippet);
      if (problem) {
        setError(translate(lang, "save_invalid", { problem }));
        return;
      }
      await api.saveSnippet(snippet, draft.category);
      const next = { snippet, category: draft.category };
      setDraft(next);
      setSaved(snapshot(next));
      await reload();
    } catch (e) {
      setError(translate(lang, "save_failed", { problem: errorText(e) }));
    } finally {
      setSaving(false);
    }
  }, [draft, lang, reload]);

  // Ctrl+S saves.
  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "s") {
        event.preventDefault();
        if (page === "snippet" && dirtyRef.current) void save();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [page, save]);

  const newSnippet = async (base?: Draft) => {
    if (!(await mayLeave())) return;
    const next: Draft = base ?? {
      snippet: { id: uuidv7(), trigger: ":", label: "", content: { blocks: [] }, enabled: true },
      category: draft?.category ?? [],
    };
    setDraft(next);
    setSaved("");
    setError(null);
    setPage("snippet");
  };

  const duplicate = () => {
    if (!draft) return;
    void newSnippet({
      snippet: {
        ...structuredClone(draft.snippet),
        id: uuidv7(),
        trigger: `${draft.snippet.trigger}2`,
        label: t("copy_of", { label: draft.snippet.label || draft.snippet.trigger }),
      },
      category: [...draft.category],
    });
  };

  const remove = async () => {
    if (!draft) return;
    const confirmed = await ask(t("delete_confirm", { label: draft.snippet.label || draft.snippet.trigger }), {
      title: "Typlet",
      kind: "warning",
    });
    if (!confirmed) return;
    const exists = library?.snippets.some((s) => s.snippet.id === draft.snippet.id);
    try {
      if (exists) await api.deleteSnippet(draft.snippet.id);
      setDraft(null);
      setSaved("");
      setPage("home");
      await reload();
    } catch (e) {
      setError(errorText(e));
    }
  };

  const switchPage = async (next: Page) => {
    if (page === "snippet" && !(await mayLeave())) return;
    if (page === "snippet") {
      setDraft(null);
      setSaved("");
    }
    setPage(next);
  };

  const others = useMemo(() => {
    const map = new Map<string, string>();
    for (const { snippet } of library?.snippets ?? []) {
      if (snippet.id !== draft?.snippet.id && snippet.enabled !== false) map.set(snippet.trigger, snippet.label || snippet.trigger);
    }
    return map;
  }, [library, draft?.snippet.id]);

  const categories = useMemo(
    () => [...new Set((library?.snippets ?? []).map((s) => s.category.join(" / ")).filter((c) => c !== ""))].sort(),
    [library],
  );

  const conflicted = useMemo(() => {
    const ids = new Set<string>();
    for (const conflict of library?.conflicts ?? []) {
      if (conflict.kind === "duplicate") conflict.snippets.forEach((id) => ids.add(id));
      else ids.add(conflict.shadowed);
    }
    return ids;
  }, [library]);

  return (
    <LangContext.Provider value={lang}>
      <div className="app">
        <Sidebar
          snippets={library?.snippets ?? []}
          selectedId={draft?.snippet.id ?? null}
          page={page}
          conflicted={conflicted}
          backgroundRunning={info?.background_running ?? true}
          onSelect={(id) => void openSnippet(id)}
          onNew={() => void newSnippet()}
          onSettings={() => void switchPage("settings")}
          onImport={() => void switchPage("import")}
          onStartBackground={() => void api.startBackground().then(() => window.setTimeout(() => void refreshInfo(), 1500))}
        />
        <main className="main">
          {settingsError && <p className="error-banner">{t("settings_unreadable", { problem: settingsError })}</p>}
          {page === "snippet" && draft && (
            <SnippetEditor
              key={draft.snippet.id}
              draft={draft}
              dirty={dirty}
              saving={saving}
              error={error}
              others={others}
              categories={categories}
              onChange={setDraft}
              onSave={() => void save()}
              onDelete={() => void remove()}
              onDuplicate={duplicate}
            />
          )}
          {page === "settings" && (
            <SettingsPage settings={settings} libraryPath={info?.library ?? ""} version={info?.version ?? ""} onCheckUpdates={() => void switchPage("update")} onSaved={(next) => {
              setSettings(next);
              setSettingsError(null);
              void refreshInfo().then(reload);
            }} />
          )}
          {page === "update" && <UpdatePage onStatus={setUpdate} />}
          {page === "import" && <ImportPage defaultDir={info?.default_espanso_dir ?? null} onImported={() => void reload()} />}
          {page === "home" && (
            <Home
              t={t}
              count={library?.snippets.length ?? 0}
              update={update?.available?.version ?? null}
              onUpdate={() => void switchPage("update")}
              errors={library?.errors ?? []}
              onNew={() => void newSnippet()}
              onImport={() => void switchPage("import")}
            />
          )}
        </main>
      </div>
    </LangContext.Provider>
  );
}

function Home(props: {
  t: (key: Key, vars?: Record<string, string | number>) => string;
  count: number;
  update: string | null;
  onUpdate: () => void;
  errors: string[];
  onNew: () => void;
  onImport: () => void;
}) {
  const { t } = props;
  return (
    <div className="page home">
      {props.update && (
        <div className="update-banner">
          <span>{t("update_banner", { version: props.update })}</span>
          <button type="button" className="primary" onClick={props.onUpdate}>
            {t("update_banner_button")}
          </button>
        </div>
      )}
      <img src="/logo.svg" alt="" width={72} height={72} />
      <h1>{t("home_title")}</h1>
      <p>{props.count === 0 ? t("home_empty") : t("home_count", { count: props.count })}</p>
      <div className="row-buttons center">
        <button type="button" className="primary" onClick={props.onNew}>
          + {t("new_snippet")}
        </button>
        <button type="button" className="secondary" onClick={props.onImport}>
          {t("import_title")}
        </button>
      </div>
      <ul className="tips">
        <li>{t("tip_trigger")}</li>
        <li>{t("tip_placeholders")}</li>
        <li>{t("tip_search")}</li>
      </ul>
      <p className="muted small">
        {t("home_donate")}{" "}
        <button type="button" className="link-button" onClick={() => void api.openLink("donate")}>
          {t("donate")}
        </button>
      </p>
      {props.errors.length > 0 && (
        <section className="panel">
          <h3>{t("library_errors")}</h3>
          <ul>
            {props.errors.map((e) => (
              <li key={e} className="mono small">
                {e}
              </li>
            ))}
          </ul>
        </section>
      )}
    </div>
  );
}
