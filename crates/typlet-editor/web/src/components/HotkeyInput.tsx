// SPDX-License-Identifier: GPL-3.0-or-later
// Records a keyboard shortcut by pressing it.

import { useState } from "react";
import { api, errorText } from "../api";
import { useLang, useT } from "../i18n";

const NAMED: Record<string, string> = {
  Space: "Space",
  Enter: "Enter",
  NumpadEnter: "Enter",
  Tab: "Tab",
  Escape: "Esc",
  Insert: "Insert",
  Delete: "Delete",
  Home: "Home",
  End: "End",
  PageUp: "PageUp",
  PageDown: "PageDown",
};

/** Turns a key event into Typlet's hotkey syntax, or null for lone modifiers. */
export function hotkeyFromEvent(event: Pick<KeyboardEvent, "code" | "ctrlKey" | "altKey" | "shiftKey" | "metaKey">): string | null {
  const { code } = event;
  let key: string | undefined;
  if (/^Key[A-Z]$/.test(code)) key = code.slice(3);
  else if (/^Digit[0-9]$/.test(code)) key = code.slice(5);
  else if (/^F([1-9]|1[0-9]|2[0-4])$/.test(code)) key = code;
  else key = NAMED[code];
  if (!key) return null;
  const parts = [];
  if (event.ctrlKey) parts.push("Ctrl");
  if (event.altKey) parts.push("Alt");
  if (event.shiftKey) parts.push("Shift");
  if (event.metaKey) parts.push("Meta");
  parts.push(key);
  return parts.join("+");
}

/** German key names as printed on German keyboards. */
export function displayHotkey(hotkey: string, german: boolean): string {
  if (!german) return hotkey;
  const names: Record<string, string> = {
    Ctrl: "Strg",
    Shift: "Umschalt",
    Meta: "Win",
    Space: "Leertaste",
    Enter: "Eingabe",
    Delete: "Entf",
    Insert: "Einfg",
    Home: "Pos1",
    End: "Ende",
    PageUp: "Bild auf",
    PageDown: "Bild ab",
  };
  return hotkey
    .split("+")
    .map((part) => names[part] ?? part)
    .join("+");
}

export function HotkeyInput(props: { value: string | null; onChange: (value: string | null) => void }) {
  const t = useT();
  const german = useLang() === "de";
  const [recording, setRecording] = useState(false);
  const [error, setError] = useState<string | null>(null);

  return (
    <span className="hotkey-input">
      <button
        type="button"
        className={`hotkey${recording ? " is-recording" : ""}`}
        onClick={() => {
          setRecording(true);
          setError(null);
        }}
        onBlur={() => setRecording(false)}
        onKeyDown={(event) => {
          if (!recording) return;
          event.preventDefault();
          if (event.key === "Escape" && !event.ctrlKey && !event.altKey) {
            setRecording(false);
            return;
          }
          const text = hotkeyFromEvent(event.nativeEvent);
          if (!text) return;
          api
            .parseHotkey(text)
            .then((normal) => {
              props.onChange(normal);
              setRecording(false);
            })
            .catch((e: unknown) => setError(errorText(e)));
        }}
      >
        {recording ? t("hotkey_press") : props.value ? displayHotkey(props.value, german) : t("hotkey_none")}
      </button>
      {props.value && (
        <button type="button" className="link-button" onClick={() => props.onChange(null)}>
          {t("hotkey_clear")}
        </button>
      )}
      {error && <span className="error">{t("hotkey_invalid")}</span>}
    </span>
  );
}
