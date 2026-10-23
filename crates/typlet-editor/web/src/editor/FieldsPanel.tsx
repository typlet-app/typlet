// SPDX-License-Identifier: GPL-3.0-or-later
// The questions of a fill-in form.

import { useT } from "../i18n";
import type { FieldDef, FieldKind } from "../types";

interface Props {
  fields: FieldDef[];
  used: string[];
  onChange: (fields: FieldDef[]) => void;
  onInsert: (name: string) => void;
  onRemove: (name: string) => void;
}

type KindName = FieldKind["type"];

function withKind(field: FieldDef, kind: KindName): FieldDef {
  const options = field.kind.type === "choice" ? field.kind.options : [];
  switch (kind) {
    case "choice":
      return { ...field, kind: { type: "choice", options: options.length > 0 ? options : [""] } };
    default:
      return { ...field, kind: { type: kind } };
  }
}

export function FieldsPanel({ fields, used, onChange, onInsert, onRemove }: Props) {
  const t = useT();
  if (fields.length === 0) return null;

  const update = (index: number, field: FieldDef) =>
    onChange(fields.map((f, i) => (i === index ? field : f)));

  return (
    <section className="panel fields" aria-label={t("form_fields")}>
      <header>
        <h3>{t("form_fields")}</h3>
        <span className="hint">{t("form_fields_help")}</span>
      </header>
      {fields.map((field, index) => (
        <div className="field-row" key={field.name}>
          <label>
            {t("field_question")}
            <input value={field.label} onChange={(e) => update(index, { ...field, label: e.target.value })} />
          </label>
          <label>
            {t("field_kind")}
            <select value={field.kind.type} onChange={(e) => update(index, withKind(field, e.target.value as KindName))}>
              <option value="text">{t("field_text")}</option>
              <option value="multiline_text">{t("field_multiline")}</option>
              <option value="choice">{t("field_choice")}</option>
              <option value="yes_no">{t("field_yes_no")}</option>
            </select>
          </label>
          <label>
            {t("field_default")}
            {field.kind.type === "yes_no" ? (
              <select
                value={field.default ?? ""}
                onChange={(e) => update(index, { ...field, default: e.target.value || undefined })}
              >
                <option value="">{t("field_default_none")}</option>
                <option value={t("yes")}>{t("yes")}</option>
                <option value={t("no")}>{t("no")}</option>
              </select>
            ) : field.kind.type === "choice" ? (
              <select
                value={field.default ?? ""}
                onChange={(e) => update(index, { ...field, default: e.target.value || undefined })}
              >
                <option value="">{t("field_default_first")}</option>
                {field.kind.options.filter((o) => o.trim() !== "").map((option) => (
                  <option key={option} value={option}>
                    {option}
                  </option>
                ))}
              </select>
            ) : (
              <input
                value={field.default ?? ""}
                onChange={(e) => update(index, { ...field, default: e.target.value || undefined })}
              />
            )}
          </label>
          {field.kind.type === "choice" ? (
            <label className="span-all">
              {t("field_options")}
              <textarea
                rows={Math.max(2, field.kind.options.length)}
                value={field.kind.options.join("\n")}
                onChange={(e) =>
                  update(index, { ...field, kind: { type: "choice", options: e.target.value.split("\n") } })
                }
              />
            </label>
          ) : null}
          <div className="field-actions">
            <button type="button" className="link-button" onClick={() => onInsert(field.name)}>
              {t("field_insert_again")}
            </button>
            <button
              type="button"
              className="link-button danger"
              onClick={() => onRemove(field.name)}
              title={used.includes(field.name) ? t("field_remove_used") : undefined}
            >
              {t("remove")}
            </button>
          </div>
        </div>
      ))}
    </section>
  );
}
