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
| Content blocker and filter lists | **Works** (`stonedgem-blocker`), including on Servo's requests and element hiding (`stonedgem-engine`) |
| History, bookmarks, reading list, closed tabs | **Works** (`stonedgem-library`); History and Bookmarks pages in the window; restoring sessions: Planned (5) |
| Rendering web pages | Spike works headless (`stonedgem-engine`); real-site compatibility not measured yet (4) |
| Tabs, address bar, start page, menu, settings, find in page | **Works** (`stonedgem-chrome`) |
| Downloads, context menu, zoom, reopening tabs | **Works** (`stonedgem-chrome`) |
| Tree tabs, help | Planned (later) |
| PDF viewing | Unsupported — Servo has no PDF viewer; needs a PDF renderer |
| Offline games | Planned (5) — they're native, not web |
| Built-in updater, AUR | Planned (8) |
| DRM video (Widevine) | Unsupported |

## Extensions

| Kind | Status |
|---|---|
| Installing, converting and checking packages (all kinds below) | **Works** (`stonedgem-extensions`); running them is what the rows below track |
| OyNIx extensions: `code.ts` + `content.css` | **Works** in the window: injected as Servo user scripts, guarded by their `matches`/`excludes` (WebExtension match patterns) and run at `run_at`; installed, switched and removed in Settings › Extensions |
| `extra.jsonl` data, `widget.tsx`, `extra.html`, `extra.md` | **Works**: data reaches content scripts and widgets; widgets are cards on the start page (each in a sandboxed frame); `extra.html` / `extra.md` open in a tab from the menu |
| `graphics.qt` panels | **Needs migration** to Slint: an extension adds `panel.slint` (checked at install), shown beside the page from the menu, with Slint's Material widgets. Callbacks reach the extension's Z#++ script (`on panel`), which sets properties with `panel_set`. With both files, StonedGem shows `panel.slint` and OyNIx `graphics.qt` |
| `theme.json` browser styles | Planned (5) — mapped to the shared design tokens |
| `theme-shape.qml` | **Needs migration** to a Slint/Iced shape description |
| Power: `network.ts` (block, headers, observe, fetch) | **Works** except headers: runs in a hidden background page; `block` adds blocker rules, `onRequest` gets every request from the engine's hook, `fetch` has no cross-origin limits. `setRequestHeader` is accepted but **not applied**: Servo's request hook can replace a response but not change an outgoing request |
| Power: `filters.txt` | **Works** (`stonedgem-blocker`) |
| Z#++ extension scripts (`script.zsp`) | **Works**: run on page loads in the sandbox (page_url, page_title, inject_css, run_js, notify, store, panel_set); answer `chrome.runtime.sendMessage` (`on message`) and panel callbacks (`on panel`). Chrome/Firefox extensions can carry one too |
| Chrome/Firefox content scripts (MV2, MV3) | **Works** for these APIs: `runtime.id`, `runtime.getManifest()`, `runtime.sendMessage()` (answered by the extension's `script.zsp`), `storage.local` (`get`/`set`/`remove`/`clear`, saved per extension in the profile, 5 MB), `tabs.create({url})` (http/https). Content scripts run guarded by `matches`/`exclude_matches` at `run_at`, with `css`. Servo has no isolated worlds, so they share the page's JavaScript world (as `"world": "MAIN"` does in Chrome): a page can see `chrome.*` where an extension runs |
| Chrome/Firefox: background pages/service workers, popups, options pages, other APIs | **Unsupported** for now: added one at a time, each only when tested. Blocking `webRequest`, `debugger`, `proxy` and DevTools pages are unlikely |

## Engine capabilities still to prove

Site compatibility on a fixed test list · WebUSB · WebRTC · media
playback breadth · printing to PDF · DevTools.
