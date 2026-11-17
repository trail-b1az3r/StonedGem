# Engine spike (phase 4): Servo

Status: **the spike works end to end, headless**, and a first survey of 12
real sites is below: documents and news work, heavy web apps don't yet. The
full survey on a home connection is still to do.

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

## First survey: 12 real sites (2026-11-17)

Run headless from a cloud machine, through its network proxy, with
`examples/sitecheck.rs`. The baseline is Chromium (headless, Playwright) on
the same machine and network, with `examples/baseline-chromium.js`.
"Text" is `document.body.innerText.length` once the page has loaded.

| Site | Servo text / links / images | Chromium text / links / images | Verdict |
|---|---|---|---|
| Wikipedia article | 29594 / 1220 / 12 | 20512 / 1224 / 12 | **Same** (Servo counts collapsed sections as text) |
| BBC News | 8348 / 165 / 21 | 8380 / 165 / 26 | **Same** |
| Hacker News | 3940 / 225 / 3 | 3935 / 225 / 3 | **Same** |
| docs.rs | 2775 / 35 / 0 | 2775 / 35 / 0 | **Same** |
| DuckDuckGo | 7742 / 126 / 46 | 774 / 30 / 4 | **Same or better** |
| GitHub repository | 5697 / 229 / 9 | 8096 / 395 / 31 | **Partial**: about 70 % of the text, 58 % of the links |
| Google sign-in | 215 / 4 / 2 | 968 / 6 / 1 | **Worse**: the page loads with less on it |
| OpenStreetMap | 0 / 0 / 0 | 565 / 42 / 25 | **Broken**: the map app doesn't start |
| YouTube | error page | 0 / 2 / 2 (app shell) | **Broken** |
| WSJ | 0 | HTTP 401 | Inconclusive: the site refuses this network |
| Amazon | 0 | HTTP 202, empty | Inconclusive: bot check |
| Reddit | 0 | 143 / 1 / 1 | Inconclusive: nearly empty in both |

So: 5 of the 9 sites that answer this network work as in Chromium, 1 works
partly, and 3 (sign-in, a map app, video) are worse or broken. It's enough to keep
going with Servo for documents, news and reference sites, but not yet for a
daily browser.

No requests were blocked on these pages. The page counts stop at the
`load` event, and the built-in rules are only a short list (no filter
lists were subscribed), so this doesn't show blocking failing; the local
test does show it working. The survey on a desktop should subscribe
EasyList and EasyPrivacy first.

## Still to do: the full survey

The plan's go/no-go is site compatibility on a fixed list of real sites
(news, shops, sign-ins, video, web apps), compared with OyNIx on
QtWebEngine. The first survey above ran from a cloud machine, where several sites
refuse the connection. The full one needs an ordinary home connection:

1. `cargo run -p stonedgem-engine --example sitecheck -- sites.txt` with a
   longer list (examples/sites.txt is a start), and EasyList/EasyPrivacy
   subscribed. Screenshots are still to add.
2. Run it on a desktop with the same list in OyNIx, and score each site
   *works / usable / broken*.
3. If too many are broken, evaluate WPE WebKit against the same list, as
   MIGRATION.md says.

Known gaps to expect from Servo: no PDF viewer, no DRM (Widevine), no
WebExtensions API (see COMPATIBILITY.md).
