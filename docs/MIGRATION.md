# Migrating OyNIx onto StonedGem

OyNIx (Rust + Qt 6/QML + Qt WebEngine) moves onto StonedGem one piece at a
time. After every phase both still build, OyNIx still ships, and its tests
still pass. Nothing is removed from OyNIx until StonedGem's replacement
works and is tested.

## Phase 1 — engine-independent services ✅

Moved from OyNIx into crates, with their tests:

| Crate | From OyNIx | What it is |
|---|---|---|
| `stonedgem-url` | `src/core/omnibox.rs` | typed text → URL or search, `!bangs`, display forms, tracking-parameter removal that never touches sign-ins |
| `stonedgem-poweroynix` | `src/core/poweroynix.rs` | `.poweroynix` signature checking (SHA3-256 / SHA-256, Ed25519); trusted keys passed in |
| `stonedgem-vault` | `src/core/passwords.rs`, `src/core/firefox.rs` | encrypted password store (folder passed in), CSV import, Firefox passwords and bookmarks |

OyNIx now depends on these crates (a git dependency pinned to a commit)
and its old modules are thin wrappers that supply OyNIx's folders and keys.

## Phase 2 — profile, blocker, network rules ✅

| Crate | From OyNIx | What it is |
|---|---|---|
| `stonedgem-profile` | `src/core/paths.rs` | profile folders under an app name the product sets (`set_app`); OyNIx sets `oynix`, so existing profiles stay put |
| `stonedgem-blocker` | `src/core/blocker.rs`, `filterlists.rs`, `netrules.rs` | the filter-list engine, the list catalogue and `network.ts` runtime rules; `should_block(request)` returns a decision, ready for any engine |

What stays with the product: which extension filter files are enabled
(passed in with `set_extension_filters`) and the HTTP client behind
`network.ts`'s cross-origin fetch.

## Phase 3 — library and extension packages ✅

| Crate | From OyNIx | What it is |
|---|---|---|
| `stonedgem-library` | `src/core/library.rs` | history, bookmarks (Netscape HTML import/export), reading list, recently closed tabs |
| `stonedgem-extensions` | `src/core/extensions.rs`, `tscompile.rs`, `material.rs` | the extension registry and converters (Chrome/Firefox MV2/MV3, `.crx`, `.xpi`, `.oynix`, `.poweroynix`), TypeScript/TSX compilation (oxc), widgets, network.ts pages, Material schemes |

The product supplies a `Host` at start-up: its name (shown as who verified
built-ins), its trusted `.poweroynix` keys, a QML checker and the engine's
page-channel script. A browser without a QML runtime leaves the checker
out and packages with QML panels are refused with a clear message.
QML checking itself (`qmlcheck.rs`) stays in OyNIx until panels move to
Slint in phase 6.

## Phase 4 — engine spike (spike ✅, site survey to do)

Done: `stonedgem-engine` renders headless with Servo 0.8, every request goes
through `stonedgem-blocker`, and extension scripts, styles and element
hiding work; see [ENGINE-SPIKE.md](ENGINE-SPIKE.md). Still to do: the
site-compatibility survey below, which needs a desktop machine.


`stonedgem-engine` wraps Servo behind StonedGem's own types. A small test
app opens a window, loads pages, routes every request through
`stonedgem-blocker`, and injects an OyNIx extension's content script. We
measure site compatibility on a fixed list of sites before going further;
if it's too poor, WPE WebKit is evaluated against the same list.

## Phase 5 — Iced chrome (window, menu, History, Bookmarks, Settings, Find ✅)

`stonedgem-chrome` builds `stonedgem-browser`, drawn entirely with Iced in
OyNIx's Material 3 colours (`stonedgem-theme`, light or dark, six accents):

- tabs, an address pill (`stonedgem-url`: addresses, searches, `!bangs`;
  "Not secure" on plain http), back/forward/reload/home, a ☆ bookmark star,
  the count of blocked requests and a ⋮ menu;
- the page: Servo's frames by readback, with mouse, wheel and keys;
- **History** (`stonedgem://history`, Ctrl+H): visits recorded by
  `stonedgem-library`, searchable, removable, clearable;
- **Bookmarks** (`stonedgem://bookmarks`, Ctrl+Shift+O, Ctrl+D to add);
- **Settings** (`stonedgem://settings`, Ctrl+,): dark/light, accent colour,
  search engine, home page, ad/tracker blocking and element hiding; saved
  in the profile as `browser.json`;
- **Find in page** (Ctrl+F): "3 of 58", ↑/↓, highlights cleared on close.

Checked under Xvfb with `crates/stonedgem-chrome/xvfb-check.sh`:
screenshots of [the page](screenshots/browser-wikipedia.png),
[find](screenshots/browser-find.png), [the menu](screenshots/browser-menu.png),
[History](screenshots/browser-history.png),
[Settings](screenshots/browser-settings.png) and
[Settings, light](screenshots/browser-settings-light.png).

Still to do in this phase: downloads (Servo's download support needs
checking first), a context menu, zoom, restoring tabs on start, and the
oynix-next package.


`stonedgem-chrome`: window, tabs, address bar (using `stonedgem-url`),
menus, settings and start page, drawing pages from the engine
(readback first). Shipped as **oynix-next**, a preview alongside OyNIx,
not a replacement.

## Phase 6 — extension runtime and Slint panels

Content scripts, `network.ts`, widgets and pages on the engine hooks;
QML panels (`graphics.qt`) rewritten as Slint windows; the `chrome.*`
APIs one by one, each with tests. [COMPATIBILITY.md](COMPATIBILITY.md) is
updated as each lands.

## Phase 7 — Z#++

A library build of Z# with a sandbox, behind a C ABI. See
[ZSHARP.md](ZSHARP.md).

## Phase 8 — OyNIx 5.4.0 on StonedGem

When oynix-next covers everyday browsing, passwords, bookmarks, the
blocker and OyNIx extensions, OyNIx 5.4.0 switches to it. The Qt build is
kept for one release for anyone who relies on Chrome/Firefox extensions
StonedGem can't run yet. Profiles are migrated on first start (history,
bookmarks, passwords, settings, extensions).
