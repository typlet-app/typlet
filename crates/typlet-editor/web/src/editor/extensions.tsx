// SPDX-License-Identifier: GPL-3.0-or-later
// Placeholders as chips in the rich-text editor: date, cursor, clipboard,
// form fields and other snippets. Node names and attributes match convert.ts.

import { mergeAttributes, Node } from "@tiptap/core";
import { NodeViewWrapper, ReactNodeViewRenderer, type ReactNodeViewProps } from "@tiptap/react";
import { createContext, useContext } from "react";
import StarterKit from "@tiptap/starter-kit";
import { Placeholder } from "@tiptap/extensions";
import { NODE } from "../convert";
import { useLang, useT } from "../i18n";
import { shiftedToday, strftime } from "../strftime";
import type { FieldDef, Snippet } from "../types";

/** The snippet's fields, so field chips can show their labels. */
export const FieldsContext = createContext<FieldDef[]>([]);

/** All snippets of the library, so included snippets show their names. */
export const SnippetsContext = createContext<Snippet[]>([]);

function Chip(props: { kind: string; icon: string; label: string; selected: boolean; title?: string }) {
  return (
    <NodeViewWrapper
      as="span"
      className={`chip chip-${props.kind}${props.selected ? " is-selected" : ""}`}
      title={props.title}
      data-drag-handle
    >
      <span className="chip-icon" aria-hidden>
        {props.icon}
      </span>
      {props.label}
    </NodeViewWrapper>
  );
}

function DateChip({ node, selected }: ReactNodeViewProps) {
  const t = useT();
  const lang = useLang();
  const format = String(node.attrs["format"] ?? "%d.%m.%Y");
  const offset = Number(node.attrs["offsetDays"] ?? 0);
  const example = strftime(format, shiftedToday(offset), lang);
  const title = offset === 0 ? t("chip_date_title") : t("chip_date_offset_title", { days: offset });
  return <Chip kind="date" icon="📅" label={example} selected={selected} title={title} />;
}

function CursorChip({ selected }: ReactNodeViewProps) {
  const t = useT();
  return <Chip kind="cursor" icon="⌶" label={t("chip_cursor")} selected={selected} title={t("chip_cursor_title")} />;
}

function ClipboardChip({ selected }: ReactNodeViewProps) {
  const t = useT();
  return (
    <Chip kind="clipboard" icon="📋" label={t("chip_clipboard")} selected={selected} title={t("chip_clipboard_title")} />
  );
}

function FieldChip({ node, selected }: ReactNodeViewProps) {
  const t = useT();
  const fields = useContext(FieldsContext);
  const name = String(node.attrs["name"] ?? "");
  const field = fields.find((f) => f.name === name);
  return (
    <Chip
      kind={field ? "field" : "field-missing"}
      icon="✎"
      label={field?.label || name}
      selected={selected}
      title={field ? t("chip_field_title") : t("chip_field_missing")}
    />
  );
}

function SnippetChip({ node, selected }: ReactNodeViewProps) {
  const t = useT();
  const snippets = useContext(SnippetsContext);
  const id = String(node.attrs["id"] ?? "");
  const included = snippets.find((s) => s.id === id);
  return (
    <Chip
      kind={included ? "snippet" : "snippet-missing"}
      icon="🧩"
      label={included ? included.label || included.trigger : t("chip_snippet_missing")}
      selected={selected}
      title={included ? t("chip_snippet_title", { trigger: included.trigger }) : t("chip_snippet_missing_title")}
    />
  );
}

function placeholderNode(name: string, view: (props: ReactNodeViewProps) => React.JSX.Element, attrs = {}) {
  return Node.create({
    name,
    group: "inline",
    inline: true,
    atom: true,
    selectable: true,
    draggable: true,
    addAttributes() {
      return attrs;
    },
    parseHTML() {
      return [{ tag: `span[data-typlet="${name}"]` }];
    },
    renderHTML({ HTMLAttributes }) {
      return ["span", mergeAttributes(HTMLAttributes, { "data-typlet": name })];
    },
    addNodeView() {
      return ReactNodeViewRenderer(view);
    },
  });
}

export const DateTimeNode = placeholderNode(NODE.dateTime, DateChip, {
  format: { default: "%d.%m.%Y" },
  offsetDays: { default: 0 },
});
export const CursorNode = placeholderNode(NODE.cursor, CursorChip);
export const ClipboardNode = placeholderNode(NODE.clipboard, ClipboardChip);
export const FieldNode = placeholderNode(NODE.field, FieldChip, { name: { default: "" } });
export const SnippetNode = placeholderNode(NODE.snippet, SnippetChip, { id: { default: "" } });

/** Everything the snippet editor needs, limited to what Typlet can store. */
export function extensions(placeholder: string) {
  return [
    StarterKit.configure({
      heading: false,
      blockquote: false,
      code: false,
      codeBlock: false,
      horizontalRule: false,
      strike: false,
      trailingNode: false,
      link: {
        openOnClick: false,
        autolink: true,
        protocols: ["mailto"],
        HTMLAttributes: { rel: "noopener noreferrer", target: null },
      },
    }),
    Placeholder.configure({ placeholder }),
    DateTimeNode,
    CursorNode,
    ClipboardNode,
    FieldNode,
    SnippetNode,
  ];
}
