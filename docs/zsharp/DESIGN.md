# Z#++ docs: design

The documentation site for Z#++ (built by `crates/stonedgem-zsharp-docs`
from the Markdown in this folder). It's a reference people read while
writing code, so the design serves long reading and scanning code: quiet
neutrals, one colour for links, real typography, and code that looks like
code. No hero gradients, glass, emoji, or rows of identical feature cards;
the index page leads with a real program and what it draws.

This direction comes from StonedGem's own look and the needs of reference
docs. It wasn't checked against a library of comparable sites.

## Colour

| Token | Light | Dark | Role |
|---|---|---|---|
| `--paper` | `#fbfaf7` | `#121417` | Page background (warm off-white / near-black) |
| `--panel` | `#f4f2ed` | `#181b1f` | Code blocks, sidebar hover |
| `--ink` | `#1c1d1f` | `#e6e7e9` | Body text (16.2:1 / 14.9:1 on the page) |
| `--muted` | `#5b5f66` | `#a2a7ae` | Secondary text, captions (6.1:1 / 7.6:1) |
| `--rule` | `#e3e0d8` | `#2a2e34` | Borders and dividers |
| `--link` | `#1d5bb8` | `#8fb4ff` | Links, current page, focus ring — StonedGem's blue (6.2:1 / 8.9:1) |
| `--note` | `#8a5a00` | `#e8b45c` | The label of a “Note” callout only |
| `--bad` | `#a3322a` | `#ff9b8f` | Error output label |

Syntax colours sit inside that palette: keywords brick (`#a3322a` /
`#ff9b8f`), strings green (`#2f6b3a` / `#9fd49f`), numbers amber
(`#8a5a00` / `#e8b45c`), built-ins and calls in the link blue, types violet
(`#6b4fa0` / `#c5b3ff`), comments muted italic (`#63676e` / `#8b9098`). Every syntax colour keeps at least 4.5:1 on the code background in both themes. Colour literals (`#f80`)
show a swatch of the colour.

## Typography

- **IBM Plex Sans** for text, **IBM Plex Mono** for code (system fallbacks).
- Body 17px / 1.65, measure about 72 characters.
- Scale: 13 (labels, captions) · 15 (code, UI) · 17 (body) · 21 (h3) · 26
  (h2) · 36 (h1). Weights 400, 500, 600.
- Section labels in the sidebar: 12px, letter-spaced, muted.

## Spacing and shape

- 4-pt scale: 4, 8, 12, 16, 24, 32, 48, 64.
- Radius 6 (code, buttons), 10 (callouts, figures). No shadows: borders
  carry the structure.
- Layout: sticky sidebar 248px · content up to 760px · “On this page”
  rail 200px from 1240px wide. Below 900px the sidebar becomes a
  collapsible “Contents” at the top.

## Components

- **Code figure**: a caption bar (“program”, “GPU code · .zsg”, or the
  file's path linked to the repository) with a Copy button; the code;
  then, for examples that run, an **Output** block and the **Canvas** the
  program drew.
- **Callouts**: blockquotes that start with **Note** or **Careful** — a
  left rule and the label in colour, nothing else.
- **Pager**: previous/next with the page names.
- **Links**: underlined in text (offset, thin); the current page in the
  sidebar is bold with a 2px link-blue rule.
- **Focus**: 2px link-blue outline, offset 2px, on everything focusable.
- **Theme**: follows the system; the Theme button cycles system → light →
  dark and remembers the choice in the browser.

## Imagery

Only what Z#++ itself draws: canvases produced by the examples when the
site is built. No stock art, icons only where text can't do the job.
