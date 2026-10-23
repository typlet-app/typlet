# 🎨 Design

How Typlet looks and speaks. The background program implements these tokens in [`theme.slint`](../crates/typlet/ui/theme.slint); the editor (M4) will use the same values.

## Principles

1. **Calm and familiar.** Native controls (Slint's Fluent style), system light/dark mode, the system accent colour for buttons.
2. **One thing per window.** Every window answers one question; a primary button says what happens next.
3. **Explain, don't blame.** Messages say what happened and what to do, in plain words. Technical details go into a smaller box below.
4. **Keyboard first, mouse friendly.** Every window works with Enter and Esc; search works with arrow keys.

## Colours

| Token | Light | Dark | Used for |
|---|---|---|---|
| `brand` → `brand-end` | `#5B5BF0` → `#A24DF2` | same | Logo, page indicator |
| `text` | system | system | Body text |
| `muted` | `#6B7280` | `#A1A1AA` | Secondary text, hints |
| `border` | `#E4E4E7` | `#3F3F46` | Window outline |
| `hover` | `#F1F0FB` | `#2E2E35` | Hovered list rows |
| `selected` | `#E8E6FD` | `#3730A3` | Selected list row |
| `chip` / `chip-text` | `#EDE9FE` / `#4C1D95` | `#312E81` / `#E0E7FF` | Trigger chips such as `:mfg` |
| `success` | `#15803D` | `#4ADE80` | "It works", passed checks |
| `warning` | `#B45309` | `#FBBF24` | Checks that need attention |
| `error` | `#B91C1C` | `#F87171` | Failures, invalid input |

Text on every background meets WCAG AA contrast.

## Shape and spacing

| Token | Value |
|---|---|
| `radius` / `radius-large` | 8 px / 12 px |
| `gap` | 8 px between related items |
| `padding` / `padding-large` | 16 px / 24 px inside windows |
| Font sizes | 12 px small, 14 px body, 20 px titles, 26 px hero |

Triggers are always shown as **chips** in a monospace font, so users learn to recognise them.

## Voice

- English is the source language; German is the first translation.
- German uses the informal **du**, short sentences and no technical terms.
- Name things by what users see: "system tray, next to the clock", not "notification area icon".

### Glossary

| English | German | Never |
|---|---|---|
| snippet | Textbaustein | Snippet, Vorlage |
| trigger | Kürzel | Trigger, Abkürzung |
| quick search | Schnellsuche | Popup |
| form | Formular | Dialog |
| system tray | Infobereich | Taskleiste |
| paste / insert | einfügen | pasten |
| set-up assistant | Einrichtungsassistent | Wizard |

## Icons

The logo ([`assets/logo.svg`](../assets/logo.svg)) is the only icon source. Tray and window icons are rendered from it by `scripts/render-icons.py`; the paused tray icon is the same shape in grey.
