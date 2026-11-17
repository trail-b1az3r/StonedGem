# Compatibility matrix

What OyNIx 5.3 can do, and where StonedGem stands on each. A row says
**works** only when StonedGem code and tests show it.

Legend: **Works** · **Planned** (phase in [MIGRATION.md](MIGRATION.md)) ·
**Needs migration** (works only after the extension is changed) ·
**Unsupported** (no plan yet) · **Unknown** (depends on engine testing).

## Browser features

| Feature | Status |
|---|---|
| Address bar: URL vs search, `!bangs`, display, tracking removal | **Works** (`stonedgem-url`) |
| Saved passwords, CSV import, Firefox import | **Works** (`stonedgem-vault`); filling forms: Planned (6) |
| `.poweroynix` signature checking | **Works** (`stonedgem-poweroynix`) |
| Content blocker and filter lists | **Works** (`stonedgem-blocker`); hooking it into Servo's requests: Planned (4) |
| History, bookmarks, reading list, sessions | Planned (3) |
| Rendering web pages | Unknown — Servo spike (4) |
| Tabs, tree tabs, start page, settings, help | Planned (5) |
| PDF viewing | Unsupported — Servo has no PDF viewer; needs a PDF renderer |
| Offline games | Planned (5) — they're native, not web |
| Built-in updater, AUR | Planned (8) |
| DRM video (Widevine) | Unsupported |

## Extensions

| Kind | Status |
|---|---|
| OyNIx extensions: `code.ts` + `content.css` | Planned (6) — user scripts/styles via Servo's `UserContentManager` |
| `extra.jsonl` data, `widget.tsx`, `extra.html`, `extra.md` | Planned (6) |
| `graphics.qt` panels | **Needs migration** to Slint (`.slint`) |
| `theme.json` browser styles | Planned (5) — mapped to the shared design tokens |
| `theme-shape.qml` | **Needs migration** to a Slint/Iced shape description |
| Power: `network.ts` (block, headers, observe, fetch) | Planned (6) — `load_web_resource` |
| Power: `filters.txt` | Planned (2) |
| Chrome/Firefox extensions (MV2, MV3) | **Unsupported** for now: Servo has no WebExtensions. APIs will be added one at a time, and an extension counts as working only when tested. Extensions built around popups, storage and content scripts are the likely first; blocking `webRequest`, `debugger`, `proxy` and DevTools pages are unlikely. |

## Engine capabilities still to prove

Site compatibility on a fixed test list · WebUSB · WebRTC · media
playback breadth · printing to PDF · DevTools.
