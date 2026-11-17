# StonedGem architecture

StonedGem is a browser platform: an engine binding, browser chrome, an
extension runtime and the services around them, as Rust crates that a
product (OyNIx, and later PowerOyNix) configures. It doesn't depend on
OyNIx; OyNIx depends on it.

This document is the proposal the brief asks for before implementation.
It separates what **exists and is tested** from what is **planned**.

## Status

| Layer | Status |
|---|---|
| Engine-independent services (address bar, `.poweroynix` verification, password vault, Firefox import) | **Exists**, moved from OyNIx with their tests. OyNIx uses these crates. |
| Engine binding (Servo) | Spike working, headless — `stonedgem-engine`; see [ENGINE-SPIKE.md](ENGINE-SPIKE.md). |
| Iced browser chrome | First window working — `stonedgem-chrome` (`stonedgem-browser`): tabs, address bar, page view. |
| Extension runtime | Planned — phase 6. |
| Slint extension UI | Planned — phase 6. |
| Z#++ scripting | Planned — phase 7. See [ZSHARP.md](ZSHARP.md). |

Nothing below the first row is implemented yet, and nothing should be
described as working until its crate and tests say so.

## Layers and boundaries

```
┌───────────────────────────── product (OyNIx, PowerOyNix) ─────────────────────────────┐
│  branding, defaults, trusted keys, built-in extensions, packaging                      │
└───────────────┬───────────────────────────────┬───────────────────────────────────────┘
                │                               │
   ┌────────────▼────────────┐      ┌───────────▼────────────┐
   │ stonedgem-chrome (Iced) │      │ stonedgem-extui (Slint)│   separate native windows
   │ window, tabs, address   │      │ popups, options,       │   (no Iced/Slint embedding)
   │ bar, menus, settings    │      │ permission dialogs     │
   └────────────┬────────────┘      └───────────┬────────────┘
                │        commands / events      │
   ┌────────────▼───────────────────────────────▼────────────┐
   │ stonedgem-runtime: tabs, sessions, profile, extensions  │
   │ (lifecycle, permissions, messaging, content scripts)    │
   └───────┬──────────────────────────┬──────────────────────┘
           │                          │
   ┌───────▼────────┐        ┌────────▼─────────────────────────────────────────┐
   │ stonedgem-     │        │ services: url ✓ · poweroynix ✓ · vault ✓ ·       │
   │ engine (Servo) │◄──────►│ blocker ✓ · profile ✓ · library ✓ · extensions ✓ │
   └───────┬────────┘        └──────────────────────────────────────────────────┘
           │
   ┌───────▼─────────────────────────────────────────┐
   │ stonedgem-platform: Linux integration (C++23,   │
   │ behind a C ABI, only where Rust can't do it)    │
   └─────────────────────────────────────────────────┘
```

✓ = exists today.

Rules for every boundary:

- **Services** know nothing about the engine or any toolkit. They take
  plain data and folders as arguments (for example the vault takes the
  folder to store logins in; signature checking takes the trusted-key list).
- **The engine crate** is the only one that names Servo types. The rest of
  StonedGem sees its own `Engine`, `View`, `LoadDecision`… types, so the
  engine could be replaced without touching the chrome or the runtime.
- **Iced and Slint never share a window.** The chrome is Iced; extension
  interfaces are Slint windows the runtime opens. They talk through the
  runtime's command/event channel, never by calling each other.
- **Native code** (C++23, D) sits behind a documented C ABI in its own crate,
  with a Rust wrapper. Nothing else links it directly.

## Engine decision: Servo

Requirements: no Chromium, no Qt; embeddable from Rust; can intercept
requests (blocker, `network.ts`), inject scripts and styles (OyNIx
extensions, cosmetic filtering), run code in pages, and render offscreen
so the chrome can draw the page.

| Option | Verdict |
|---|---|
| **Servo** (Rust, MPL-2.0) | **Chosen.** Published on crates.io as `servo` (0.8.0 in its docs at the time of writing). Its embedding API has a `WebViewDelegate` with `load_web_resource` (intercept and replace HTTP(S) loads), `request_navigation`, `request_permission`, console messages; a `UserContentManager` for injected scripts and styles; `WebView::evaluate_javascript`; and window, offscreen and software rendering contexts. |
| WebKitGTK / WPE WebKit | Mature, but C with GObject; WebKitGTK is a GTK widget (doesn't fit an Iced window), WPE needs its own platform plumbing. Kept as the fallback if Servo's site compatibility proves too limited. |
| Gecko | No supported embedding API. |
| Ladybird (LibWeb) | Promising, but no stable embedding API and C++-only. |
| Our own engine | Out of scope: years of work before ordinary sites render. Not pretended. |

Licensing: Servo is MPL-2.0 (file-level copyleft). Using it as a
dependency is compatible with StonedGem and OyNIx being MIT; changes to
Servo's own files would stay MPL-2.0.

Known limits to plan around:

- Site compatibility is well behind Chromium; some sites will break.
- **No WebExtensions support.** Everything extension-related is ours to
  build on top of the hooks above (see [COMPATIBILITY.md](COMPATIBILITY.md)).
- No built-in PDF viewer, password manager, or DRM (Widevine).

## Iced chrome and the page

Iced draws with wgpu; Servo renders through its own rendering context.
Plan, in order of effort:

1. **Readback:** render into Servo's offscreen or software context, read
   the pixels, upload them as an Iced image each frame. Simple and
   portable; costs a copy per frame.
2. **Shared texture:** hand Servo a custom `RenderingContext` that draws into
   a texture wgpu can sample, avoiding the copy. Platform-specific
   (EGL/dmabuf on Linux). Only if readback is too slow.

Input goes the other way: Iced pointer and key events → `notify_input_event`.

## Extensions

The runtime implements extension behaviour itself, on the engine hooks:

| OyNIx feature | Engine hook |
|---|---|
| OyNIx extensions (`code.ts`, `content.css`) | `UserContentManager` user scripts/styles |
| Content blocker, `filters.txt` | `load_web_resource` (cancel or replace) |
| `network.ts` headers / block / observe | `load_web_resource` |
| `network.ts` fetch | runtime's own HTTP client |
| Panels (`graphics.qt`) | Rewritten as Slint (`.slint`) windows |
| Pages (`extra.html`, `extra.md`) | A `WebView` in its own window |
| Widgets (`widget.tsx`) | A small `WebView` on the start page |
| Chrome/Firefox `chrome.*` APIs | Ours to implement, API by API, with tests |

Slint is not an HTML renderer: HTML extension pages are shown by the engine,
and only native QML panels move to Slint.

## Visual design: Tactile Liquid Expressive

One set of design tokens (colour roles, spacing, radii, type scale, motion)
lives in a shared crate and is read by both the Iced theme and generated
Slint globals, so both toolkits look the same. Translucent surfaces fall
back to opaque where the compositor can't blur; motion respects the
reduced-motion setting.

## Languages

- **Rust**: everything by default.
- **C++23**: only platform integration Rust can't reasonably do, behind a C ABI.
- **D**: allowed for a self-contained component with a C ABI; none is chosen
  yet. A candidate is an offline filter-list compiler.
- **Z#** ([Sam-astro's Z-Sharp](https://github.com/sam-astro/Z-Sharp), MIT):
  the basis of Z#++, a sandboxed scripting language for user automation.
  See [ZSHARP.md](ZSHARP.md).

## Platforms

Linux first: Arch (PKGBUILD, AUR) and Nix (a flake with a dev shell and a
package). Distribution packaging lives under `packaging/`, never in crates.
