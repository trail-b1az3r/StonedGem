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

## Phase 5 — Iced chrome ✅

`stonedgem-chrome` builds `stonedgem-browser` (and, run as `oynix-next`,
**OyNIx Next**, with its own profile), drawn entirely with Iced to
[DESIGN.md](DESIGN.md): Material 3 colour roles from `stonedgem-theme`, a
floating translucent panel over a gradient backdrop, raised/inset touches,
40 px targets, light and dark, six accents.

- Tabs, address capsule (`stonedgem-url`), back/forward/reload/home, ☆,
  blocked count, zoom badge, downloads button, ⋮ menu.
- Pages: Servo frames by readback; mouse, wheel, keys; **right-click menu**
  (links, images, selection); **zoom** (Ctrl+= / − / 0).
- **History**, **Bookmarks**, **Downloads** (Servo has no downloads; the
  window fetches them itself) and **Settings** pages, drawn with Iced.
- **Find in page**; **tabs reopen on start**.
- **oynix-next package**: `scripts/package-oynix-next.sh`, a .desktop
  entry, and the *OyNIx Next* workflow (manual or `oynix-next-v*` tags).

Checked under Xvfb with `crates/stonedgem-chrome/xvfb-check.sh`, which
drives every page and takes the screenshots in [screenshots/](screenshots/).

## Phase 6 — extension runtime and Slint panels (in progress)

Done: OyNIx-format extensions' content scripts and styles run in the
window. `stonedgem-extensions::matching` implements WebExtension match
patterns (in Rust, and as a JavaScript guard so a script stays correct
after the page navigates); `registry::page_scripts` gives each enabled
extension's script wrapped in its guard and `run_at`. Settings ›
Extensions installs an unpacked `.oynix` folder, switches extensions on
and off, and removes them. Tested end to end in Servo: the matching
extension runs after the DOM is ready, another site's doesn't, its
`content.css` applies.

Next: `network.ts` on the engine's request hook, widgets and extension
pages, then QML panels as Slint windows and the `chrome.*` APIs.

Content scripts, `network.ts`, widgets and pages on the engine hooks;
QML panels (`graphics.qt`) rewritten as Slint windows; the `chrome.*`
APIs one by one, each with tests. [COMPATIBILITY.md](COMPATIBILITY.md) is
updated as each lands.

## Phase 7 — Z#++ ✅

`stonedgem-zsharp`: Z# kept compatible and greatly extended (see
[ZSHARP.md](ZSHARP.md)) — `.zsp` programs, `.zsx` headers checked against
them, `.zsg` GPU kernels and fragment shaders compiled to validated WGSL
and runnable on the CPU, a 2D canvas, a sandbox, and the `zspp` tool.
Extensions can ship `script.zsp` (run on page loads with a small API); the
window draws its start-page art with Z#++ and has a playground at
`stonedgem://zsharp`.

## Phase 8 — OyNIx 5.4.0 on StonedGem

When oynix-next covers everyday browsing, passwords, bookmarks, the
blocker and OyNIx extensions, OyNIx 5.4.0 switches to it. The Qt build is
kept for one release for anyone who relies on Chrome/Firefox extensions
StonedGem can't run yet. Profiles are migrated on first start (history,
bookmarks, passwords, settings, extensions).
