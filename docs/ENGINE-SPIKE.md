# Engine spike (phase 4): Servo

Status: **the spike works end to end, headless**. Site compatibility on
real websites is the part still to measure (below).

## What was built

`crates/stonedgem-engine` wraps Servo 0.8.0 behind StonedGem's own types
(`Engine`, `Page`, `Load`), so nothing above it touches Servo:

| Piece | How |
|---|---|
| Rendering | `SoftwareRenderingContext` (off-screen, Mesa's EGL); a window comes with the Iced chrome in phase 5 |
| Blocking | `WebViewDelegate::load_web_resource` asks `stonedgem-blocker::should_block` for every HTTP(S) request; a blocked load is intercepted and cancelled, so it fails like a network error, as in OyNIx |
| Request types | Servo's `Destination` mapped to the blocker's type numbers (document, subdocument, script, image, style, font, media, XHR, ping…) |
| Extension scripts and styles | `UserContentManager` user scripts and user style sheets |
| Element hiding | the blocker's selectors for the page's host become one user style sheet |
| Running JS | `Page::eval`, waiting on Servo's event loop |

## Test (`crates/stonedgem-engine/tests/spike.rs`)

A local HTTP server serves a page with an allowed script, an ad script
(`doubleclick.net`), a tracker image (`google-analytics.com`) and an element
named by a hiding rule. The test passes when:

- the page finishes loading and its title is read;
- the allowed script ran, and was not blocked;
- both the ad and the tracker were blocked (they never leave the engine);
- the extension's content script ran and its style applied;
- the hidden element has `display: none`.

Result: **passes** (≈4 s), on Linux x86-64, 4 cores, no GPU and no display.

```sh
cargo test -p stonedgem-engine      # first build ≈5 min with 4 cores
```

## Costs

- Build: about 5 minutes from clean for `cargo test -p stonedgem-engine`
  (debug, 4 cores). SpiderMonkey comes prebuilt.
- Disk: the workspace `target/` grows to about 10 GB in debug.
- Servo's networking links `rusqlite` 0.38 (SQLite), so the whole workspace
  uses 0.38: two crates can't link SQLite at different versions.
- The engine is kept out of `default-members`, so ordinary `cargo test`
  and CI stay fast.

## Not yet known: real sites

The plan's go/no-go is site compatibility on a fixed list of real sites
(news, shops, sign-ins, video, web apps), compared with OyNIx on
QtWebEngine. That needs a machine with an ordinary internet connection and
a display. This sandbox reaches the web only through a proxy that Servo
isn't set up to use. The next step:

1. `stonedgem-engine` example `sitecheck`: load each URL from a list, wait
   for `Complete` (or 30 s), record the title, errors in the console,
   blocked requests and a screenshot.
2. Run it on a desktop with the same list in OyNIx, and score each site
   *works / usable / broken*.
3. If too many are broken, evaluate WPE WebKit against the same list, as
   MIGRATION.md says.

Known gaps to expect from Servo: no PDF viewer, no DRM (Widevine), no
WebExtensions API (see COMPATIBILITY.md).
