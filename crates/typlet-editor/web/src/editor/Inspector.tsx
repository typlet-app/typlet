// SPDX-License-Identifier: GPL-3.0-or-later
// Settings of the selected placeholder chip (date format, field, included
// snippet).

import type { Editor } from "@tiptap/core";
import { NodeSelection } from "@tiptap/pm/state";
import { useEditorState } from "@tiptap/react";
import { NODE } from "../convert";
import { useLang, useT } from "../i18n";
import { DATE_PRESETS, shiftedToday, strftime } from "../strftime";
import type { FieldDef, Snippet } from "../types";

interface Props {
  editor: Editor;
  fields: FieldDef[];
  /** The snippets an included snippet can be switched to. */
  includable: Snippet[];
  /** All snippets, to name the one already chosen. */
  snippets: Snippet[];
  onOpenSnippet: (id: string) => void;
}

export function Inspector({ editor, fields, includable, snippets, onOpenSnippet }: Props) {
  const t = useT();
  const lang = useLang();
  const selected = useEditorState({
    editor,
    selector: ({ editor: e }) => {
      const selection = e.state.selection;
      if (!(selection instanceof NodeSelection)) return null;
      return { type: selection.node.type.name, attrs: selection.node.attrs, pos: selection.from };
    },
  });
  if (!selected) return null;

  const update = (attrs: Record<string, unknown>) =>
    editor.chain().setNodeSelection(selected.pos).updateAttributes(selected.type, attrs).run();

  if (selected.type === NODE.dateTime) {
    const format = String(selected.attrs["format"] ?? "%d.%m.%Y");
    const offset = Number(selected.attrs["offsetDays"] ?? 0);
    const isPreset = (DATE_PRESETS as readonly string[]).includes(format);
    return (
      <div className="inspector" role="group" aria-label={t("date_settings")}>
        <label>
          {t("date_format")}
          <select
            value={isPreset ? format : "custom"}
            onChange={(e) => update({ format: e.target.value === "custom" ? `${format} ` : e.target.value })}
          >
            {DATE_PRESETS.map((preset) => (
              <option key={preset} value={preset}>
                {strftime(preset, shiftedToday(offset), lang)}
              </option>
            ))}
            <option value="custom">{t("date_custom")}</option>
          </select>
        </label>
        {!isPreset && (
          <label>
            {t("date_pattern")}
            <input
              className="mono"
              value={format}
              onChange={(e) => update({ format: e.target.value })}
              title={t("date_pattern_help")}
            />
          </label>
        )}
        <label>
          {t("date_offset")}
          <input
            type="number"
            value={offset}
            min={-3650}
            max={3650}
            onChange={(e) => update({ offsetDays: Math.trunc(Number(e.target.value) || 0) })}
          />
        </label>
        <span className="hint">{t("date_offset_help")}</span>
      </div>
    );
  }

  if (selected.type === NODE.field) {
    const name = String(selected.attrs["name"] ?? "");
    return (
      <div className="inspector" role="group" aria-label={t("field_settings")}>
        <label>
          {t("chip_field_shows")}
          <select value={name} onChange={(e) => update({ name: e.target.value })}>
            {!fields.some((f) => f.name === name) && <option value={name}>{name}</option>}
            {fields.map((field) => (
              <option key={field.name} value={field.name}>
                {field.label || field.name}
              </option>
            ))}
          </select>
        </label>
      </div>
    );
  }

  if (selected.type === NODE.snippet) {
    const id = String(selected.attrs["id"] ?? "");
    const current = snippets.find((s) => s.id === id);
    return (
      <div className="inspector" role="group" aria-label={t("snippet_settings")}>
        <label>
          {t("chip_snippet_shows")}
          <select value={id} onChange={(e) => update({ id: e.target.value })}>
            {!includable.some((s) => s.id === id) && (
              <option value={id}>{current ? current.label || current.trigger : t("chip_snippet_missing")}</option>
            )}
            {includable.map((s) => (
              <option key={s.id} value={s.id}>
                {s.label || s.trigger} ({s.trigger})
              </option>
            ))}
          </select>
        </label>
        {current && (
          <button type="button" className="secondary" onClick={() => onOpenSnippet(id)}>
            {t("snippet_open")}
          </button>
        )}
        <span className="hint">{t("snippet_include_help")}</span>
      </div>
    );
  }

  return null;
}
