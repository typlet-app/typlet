// SPDX-License-Identifier: GPL-3.0-or-later
import { describe, expect, it } from "vitest";
import { displayHotkey, hotkeyFromEvent } from "./HotkeyInput";

const key = (code: string, mods: Partial<Record<"ctrlKey" | "altKey" | "shiftKey" | "metaKey", boolean>> = {}) => ({
  code,
  ctrlKey: false,
  altKey: false,
  shiftKey: false,
  metaKey: false,
  ...mods,
});

describe("hotkey recording", () => {
  it("builds Typlet's syntax from key codes", () => {
    expect(hotkeyFromEvent(key("Space", { ctrlKey: true, shiftKey: true }))).toBe("Ctrl+Shift+Space");
    expect(hotkeyFromEvent(key("KeyM", { ctrlKey: true, altKey: true }))).toBe("Ctrl+Alt+M");
    expect(hotkeyFromEvent(key("F5"))).toBe("F5");
    expect(hotkeyFromEvent(key("ControlLeft", { ctrlKey: true }))).toBeNull();
  });

  it("shows German key names", () => {
    expect(displayHotkey("Ctrl+Shift+Space", true)).toBe("Strg+Umschalt+Leertaste");
    expect(displayHotkey("Ctrl+Shift+Space", false)).toBe("Ctrl+Shift+Space");
  });
});
