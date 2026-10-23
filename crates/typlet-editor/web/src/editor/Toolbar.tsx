// SPDX-License-Identifier: GPL-3.0-or-later
// Formatting buttons and the "insert placeholder" buttons.

import type { Editor } from "@tiptap/core";
import { useEditorState } from "@tiptap/react";
import { NODE } from "../convert";
import { useT } from "../i18n";

interface Props {
  editor: Editor;
  hasCursor: boolean;
  onAddField: () => void;
  onLink: () => void;
}

function Button(props: {
  label: string;
  title: string;
  active?: boolean;
  disabled?: boolean;
  onClick: () => void;
  className?: string;
}) {
  return (
    <button
      type="button"
      className={`tool${props.active ? " is-active" : ""}${props.className ? ` ${props.className}` : ""}`}
      title={props.title}
      aria-label={props.title}
      aria-pressed={props.active}
      disabled={props.disabled}
      onMouseDown={(event) => event.preventDefault()}
      onClick={props.onClick}
    >
      {props.label}
    </button>
  );
}

export function Toolbar({ editor, hasCursor, onAddField, onLink }: Props) {
  const t = useT();
  const state = useEditorState({
    editor,
    selector: ({ editor: e }) => ({
      bold: e.isActive("bold"),
      italic: e.isActive("italic"),
      underline: e.isActive("underline"),
      link: e.isActive("link"),
      bullet: e.isActive("bulletList"),
      ordered: e.isActive("orderedList"),
    }),
  });
  const chain = () => editor.chain().focus();

  return (
    <div className="toolbar" role="toolbar" aria-label={t("toolbar")}>
      <div className="tool-group">
        <Button label="B" className="tool-bold" title={t("bold")} active={state.bold} onClick={() => chain().toggleBold().run()} />
        <Button label="I" className="tool-italic" title={t("italic")} active={state.italic} onClick={() => chain().toggleItalic().run()} />
        <Button label="U" className="tool-underline" title={t("underline")} active={state.underline} onClick={() => chain().toggleUnderline().run()} />
        <Button label="🔗" title={t("link")} active={state.link} onClick={onLink} />
      </div>
      <div className="tool-group">
        <Button label="•" title={t("bullet_list")} active={state.bullet} onClick={() => chain().toggleBulletList().run()} />
        <Button label="1." title={t("ordered_list")} active={state.ordered} onClick={() => chain().toggleOrderedList().run()} />
      </div>
      <div className="tool-group tool-insert">
        <span className="tool-caption">{t("insert")}</span>
        <Button
          label={`📅 ${t("insert_date")}`}
          title={t("insert_date_title")}
          onClick={() => chain().insertContent({ type: NODE.dateTime, attrs: { format: "%d.%m.%Y", offsetDays: 0 } }).run()}
        />
        <Button
          label={`⌶ ${t("insert_cursor")}`}
          title={hasCursor ? t("insert_cursor_used") : t("insert_cursor_title")}
          disabled={hasCursor}
          onClick={() => chain().insertContent({ type: NODE.cursor }).run()}
        />
        <Button
          label={`📋 ${t("insert_clipboard")}`}
          title={t("insert_clipboard_title")}
          onClick={() => chain().insertContent({ type: NODE.clipboard }).run()}
        />
        <Button label={`✎ ${t("insert_field")}`} title={t("insert_field_title")} onClick={onAddField} />
      </div>
    </div>
  );
}
