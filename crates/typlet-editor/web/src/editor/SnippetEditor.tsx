// SPDX-License-Identifier: GPL-3.0-or-later
// Editing one snippet: name, trigger, category, rich text with placeholders,
// form fields, advanced options and a live preview.

import { EditorContent, useEditor } from "@tiptap/react";
import { useEffect, useMemo, useRef, useState } from "react";
import { api, errorText } from "../api";
import { blocksSaving, triggerIssues, type TriggerIssue } from "../conflicts";
import { fromTiptap, hasCursor, NODE, toTiptap, usedFields } from "../convert";
import { useT, type Key } from "../i18n";
import type { FieldDef, InsertMode, Preview, Snippet } from "../types";
import { extensions, FieldsContext } from "./extensions";
import { FieldsPanel } from "./FieldsPanel";
import { HotkeyInput } from "../components/HotkeyInput";
import { Inspector } from "./Inspector";
import { Toolbar } from "./Toolbar";

export interface Draft {
  snippet: Snippet;
  category: string[];
}

interface Props {
  draft: Draft;
  dirty: boolean;
  saving: boolean;
  error: string | null;
  /** Enabled triggers of all other snippets, with their labels. */
  others: Map<string, string>;
  categories: string[];
  onChange: (draft: Draft) => void;
  onSave: () => void;
  onDelete: () => void;
  onDuplicate: () => void;
}

function issueText(t: ReturnType<typeof useT>, issue: TriggerIssue): string {
  switch (issue.kind) {
    case "duplicate":
      return t("trigger_duplicate", { other: issue.other });
    case "shadowed_by":
      return t("trigger_shadowed_by", { other: issue.other });
    case "shadows":
      return t("trigger_shadows", { other: issue.other });
    default:
      return t(`trigger_${issue.kind}` as Key);
  }
}

function nextFieldName(fields: FieldDef[]): string {
  let n = fields.length + 1;
  while (fields.some((f) => f.name === `field${n}`)) n++;
  return `field${n}`;
}

export function SnippetEditor(props: Props) {
  const { draft, onChange } = props;
  const t = useT();
  const snippet = draft.snippet;
  const fields = useMemo(() => snippet.fields ?? [], [snippet.fields]);
  const [preview, setPreview] = useState<Preview | null>(null);
  const [previewError, setPreviewError] = useState<string | null>(null);
  const [showAdvanced, setShowAdvanced] = useState(false);
  const latest = useRef(draft);
  latest.current = draft;

  const editor = useEditor({
    extensions: extensions(t("editor_placeholder")),
    content: toTiptap(snippet.content),
    onUpdate: ({ editor: e }) => {
      const current = latest.current;
      onChange({ ...current, snippet: { ...current.snippet, content: fromTiptap(e.getJSON()) } });
    },
  });

  // Load the text when another snippet is opened.
  useEffect(() => {
    if (editor && !editor.isDestroyed) {
      editor.commands.setContent(toTiptap(latest.current.snippet.content), { emitUpdate: false });
    }
    // Only when the snippet changes, not on every edit.
  }, [editor, snippet.id]);

  // Live preview, rendered by the same code that inserts snippets.
  useEffect(() => {
    const timer = window.setTimeout(() => {
      api
        .preview(normalised(snippet))
        .then((result) => {
          setPreview(result);
          setPreviewError(null);
        })
        .catch((error: unknown) => setPreviewError(errorText(error)));
    }, 250);
    return () => window.clearTimeout(timer);
  }, [snippet]);

  const issues = triggerIssues(snippet.trigger, props.others);
  const blocking = issues.some(blocksSaving);
  const used = usedFields(snippet.content);
  const cursorUsed = hasCursor(snippet.content);

  const setSnippet = (changes: Partial<Snippet>) => onChange({ ...draft, snippet: { ...snippet, ...changes } });
  const setFields = (next: FieldDef[]) => setSnippet({ fields: next });

  const addField = () => {
    const name = nextFieldName(fields);
    const field: FieldDef = { name, label: t("new_field_label"), kind: { type: "text" } };
    setFields([...fields, field]);
    editor?.chain().focus().insertContent({ type: NODE.field, attrs: { name } }).run();
  };

  const insertField = (name: string) =>
    editor?.chain().focus().insertContent({ type: NODE.field, attrs: { name } }).run();

  const removeField = (name: string) => {
    if (editor) {
      // Remove the field's chips too, so the snippet stays valid.
      const positions: number[] = [];
      editor.state.doc.descendants((node, pos) => {
        if (node.type.name === NODE.field && node.attrs["name"] === name) positions.push(pos);
      });
      let tr = editor.state.tr;
      for (const pos of positions.reverse()) tr = tr.delete(pos, pos + 1);
      editor.view.dispatch(tr);
    }
    setFields(fields.filter((f) => f.name !== name));
  };

  const [link, setLink] = useState<string | null>(null);
  const openLink = () => {
    if (!editor) return;
    setLink(String(editor.getAttributes("link")["href"] ?? "") || "https://");
  };
  const applyLink = (href: string | null) => {
    if (!editor) return;
    const chain = editor.chain().focus().extendMarkRange("link");
    const url = href?.trim() ?? "";
    if (url === "" || url === "https://") chain.unsetLink().run();
    else chain.setLink({ href: url }).run();
    setLink(null);
  };

  return (
    <FieldsContext.Provider value={fields}>
      <form
        className="snippet-editor"
        onSubmit={(event) => {
          event.preventDefault();
          if (!blocking) props.onSave();
        }}
      >
        <header className="editor-header">
          <input
            className="title-input"
            value={snippet.label}
            placeholder={t("name_placeholder")}
            aria-label={t("name")}
            onChange={(e) => setSnippet({ label: e.target.value })}
          />
          <label className="switch" title={t("enabled_title")}>
            <input
              type="checkbox"
              checked={snippet.enabled !== false}
              onChange={(e) => setSnippet({ enabled: e.target.checked })}
            />
            <span>{t("enabled")}</span>
          </label>
          <button type="button" className="secondary" onClick={props.onDuplicate}>
            {t("duplicate")}
          </button>
          <button type="button" className="secondary danger" onClick={props.onDelete}>
            {t("delete")}
          </button>
          <button type="submit" className="primary" disabled={blocking || props.saving || !props.dirty}>
            {props.saving ? t("saving") : props.dirty ? t("save") : t("saved")}
          </button>
        </header>

        {props.error && <p className="error-banner">{props.error}</p>}

        <div className="meta-row">
          <label className="trigger-field">
            {t("trigger")}
            <input
              className="mono"
              value={snippet.trigger}
              // A new snippet starts with its trigger.
              autoFocus={props.dirty && snippet.label === "" && snippet.trigger === ":"}
              placeholder=":abc"
              spellCheck={false}
              onChange={(e) => setSnippet({ trigger: e.target.value })}
            />
          </label>
          <label className="category-field">
            {t("category")}
            <input
              list="categories"
              value={draft.category.join(" / ")}
              placeholder={t("category_none")}
              onChange={(e) =>
                onChange({
                  ...draft,
                  category: e.target.value
                    .split("/")
                    .map((part) => part.trim())
                    .filter((part) => part !== ""),
                })
              }
            />
            <datalist id="categories">
              {props.categories.map((c) => (
                <option key={c} value={c} />
              ))}
            </datalist>
          </label>
        </div>
        <ul className="issues" aria-live="polite">
          {issues.map((issue) => (
            <li key={`${issue.kind}-${"other" in issue ? issue.other : ""}`} className={blocksSaving(issue) ? "error" : "warning"}>
              {issueText(t, issue)}
            </li>
          ))}
          {issues.length === 0 && <li className="hint">{t("trigger_tip")}</li>}
        </ul>

        {editor && (
          <div className="text-editor">
            <Toolbar editor={editor} hasCursor={cursorUsed} onAddField={addField} onLink={openLink} />
            {link !== null && (
              <div className="inspector" role="group" aria-label={t("link")}>
                <label className="grow">
                  {t("link_address")}
                  <input
                    autoFocus
                    value={link}
                    onChange={(e) => setLink(e.target.value)}
                    onKeyDown={(e) => {
                      if (e.key === "Enter") {
                        e.preventDefault();
                        applyLink(link);
                      } else if (e.key === "Escape") {
                        setLink(null);
                      }
                    }}
                  />
                </label>
                <button type="button" className="secondary" onClick={() => applyLink(link)}>
                  {t("apply")}
                </button>
                <button type="button" className="link-button" onClick={() => applyLink(null)}>
                  {t("link_remove")}
                </button>
              </div>
            )}
            <Inspector editor={editor} fields={fields} />
            <EditorContent editor={editor} className="text-area" />
          </div>
        )}

        <FieldsPanel fields={fields} used={used} onChange={setFields} onInsert={insertField} onRemove={removeField} />

        <section className="panel preview" aria-label={t("preview")}>
          <header>
            <h3>{t("preview")}</h3>
            {preview && (
              <span className="badge">
                {preview.method === "type" ? t("method_type") : preview.formatted ? t("method_paste_formatted") : t("method_paste")}
              </span>
            )}
          </header>
          {previewError ? (
            <p className="error">{previewError}</p>
          ) : preview && preview.plain !== "" ? (
            // The HTML comes from Typlet's own renderer, which escapes all text
            // and only emits safe links.
            <div className="preview-body" dangerouslySetInnerHTML={{ __html: preview.html }} />
          ) : (
            <p className="hint">{t("preview_empty")}</p>
          )}
        </section>

        <section className="panel advanced">
          <button type="button" className="link-button" onClick={() => setShowAdvanced(!showAdvanced)} aria-expanded={showAdvanced}>
            {showAdvanced ? "▾" : "▸"} {t("advanced")}
          </button>
          {showAdvanced && (
            <div className="advanced-body">
              <label>
                {t("insert_mode")}
                <select
                  value={snippet.insert_mode ?? "auto"}
                  onChange={(e) => setSnippet({ insert_mode: e.target.value as InsertMode })}
                >
                  <option value="auto">{t("insert_mode_auto")}</option>
                  <option value="type">{t("insert_mode_type")}</option>
                  <option value="paste">{t("insert_mode_paste")}</option>
                </select>
              </label>
              <label>
                {t("snippet_hotkey")}
                <HotkeyInput value={snippet.hotkey ?? null} onChange={(hotkey) => setSnippet({ hotkey })} />
              </label>
              <span className="hint">{t("snippet_hotkey_help")}</span>
            </div>
          )}
        </section>
      </form>
    </FieldsContext.Provider>
  );
}

/** Cleans up what the form holds before it is checked or saved. */
export function normalised(snippet: Snippet): Snippet {
  const fields = (snippet.fields ?? []).map((field) =>
    field.kind.type === "choice"
      ? {
          ...field,
          kind: {
            type: "choice" as const,
            options: field.kind.options.map((o) => o.trim()).filter((o) => o !== ""),
          },
        }
      : field,
  );
  const result: Snippet = { ...snippet, label: snippet.label.trim() || snippet.trigger, fields };
  if (!result.hotkey) delete result.hotkey;
  if (result.insert_mode === "auto") delete result.insert_mode;
  return result;
}

