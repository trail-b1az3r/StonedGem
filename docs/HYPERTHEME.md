# Hyperthemes and Theme Studio

![Theme Studio editing the Neubrutalism graph, with its live preview](screenshots/theme-studio.png)

A **`.hypertheme`** restyles the whole browser: the Iced window (toolbar,
tabs, address field, cards, buttons, menus), the start page, and extension
pages. OyNIx can read the same file for its Qt chrome. It's a zip archive,
or a folder while you work on it, holding `hypertheme.toml` and any of
these parts:

| File | Language | What it gives |
|---|---|---|
| `tokens.json` | JSON | colour and number tokens; `"panels"`; `"dark"` overrides |
| `style.rs` | basic Rust | `pub const NAME: Type = value;` constants and `pub mod dark { … }`. It's parsed, never compiled. |
| `theme.zsp` | Z#++ | `func tokens(dark)` returns a map of tokens |
| `theme.zxx` | Zsharpend | `fn tokens(dark)` returns a map of tokens |
| `iced.toml` | TOML | the window's panels: `[card] fill = "#fff" shadow_x = 6`, plus `[dark.card]` |
| `pages.css` | CSS | extra rules for the start page and extension pages |
| `start.tsx` | TSX | the start page, `export default function Start()`, with `theme` and `oynix` as globals |
| `chrome.qml` | QML | OyNIx's `Theme` override; StonedGem carries it but doesn't read it |
| `background.zsg` | Z#++ shader | `fragment background(uv: vec2) -> color`, drawn behind the start page |
| `graph.json` | JSON | the Theme Studio graph it was made from |

```toml
# hypertheme.toml
[hypertheme]
name = "Neubrutalism"
version = "1.0.0"
author = "StonedGem"
description = "Cream paper, loud colour, thick ink borders and hard offset shadows."
modes = "both"        # light, dark or both
```

Theme programs can also use code kept beside them:

| Folder | What it holds |
|---|---|
| `modules/*.zxx` | Zsharpend modules: `import palette` finds `modules/palette.zxx` |
| `include/*.zsx` | Z#++ headers, for `#include` / `import "include/x.zsx"` |
| `images/*` | pictures (PNG, JPEG, WebP, GIF, SVG), such as background images |

The programs read only the theme's own files.

## Power hyperthemes (`.phypertheme`)

A theme with `power = true` in `hypertheme.toml` can change more than tokens. It installs as `.phypertheme`, and because its QML runs inside the browser's interface, installing one asks you first, as a power extension does.

| File | What it does (OyNIx) |
|---|---|
| `global.zxx` | `fn global(dark)` returns global settings: `glass` (`blur`, `tint`, `opacity`), `background_image` (a path under `images/`), `overlay_opacity`, `motion`, and anything `effect.qml` or `global.qml` reads (as `theme.global`). It may also return `tokens` and `panels`, merged like the other parts. |
| `global.zsg` | `fragment global(uv: vec2) -> color`, laid over the whole window. Its alpha decides how much shows. |
| `global.qml` | QML layers over the whole interface (or under it, with `property bool behind: true`): glass panes, frames, particles. It gets `theme` and `ui`, and is not interactive. |
| `effect.qml` | A layer effect applied to the whole rendered interface, for colour grading, blur, glow or distortion. Its root gets `source` (the interface) as a `layer.effect` does. |
| `format-panel.qt` | QML `QtObject` whose properties set the interface's format: `toolbarHeight`, `tabHeight`, `sidebarWidth`, `fontSize`, `smallFont`, `titleSize`, `headlineSize`, `displaySize`, `cornerScale`, `spacing`. |
| `styles.tsx` | TSX exporting per-component styles: `export const button = { radius: 4, weight: 700 }`, and likewise `card`, `field`, `tab`, `text`. A `css` export is added to pages. |
| `animate.zxx` | `fn animate(dark)` returns the interface's motion: `speed` (1 is normal, 0 turns motion off), `easing` (a Qt easing name such as `"OutCubic"`), `hover: { scale }`, `press: { scale }`, `page: { enter, duration }` (`enter` is `"fade"`, `"rise"`, `"zoom"`, `"slide"` or `"none"`), `tabs: { open, duration }` (`"pop"`, `"slide"`, `"fade"`, `"none"`), `menus: { open, duration }` (`"scale"`, `"fade"`, `"drop"`, `"none"`), and `shader: { duration, frames, width, height, opacity }` for `animations.zsg`. Durations are in milliseconds. |
| `animations.zsg` | `fragment animations(uv: vec2, uniform t: float) -> color`, looped over the whole window. `t` runs from 0 to 1 once per loop, so a loop that ends where it starts is seamless. It's drawn once into a sprite sheet (`shader.frames` frames of `shader.width`×`shader.height`, scaled up) and played back. |
| `animations.qml` | QML animations: a layer over the interface (not interactive) that gets `theme`, `ui`, `window` and `anim` (what `animate.zxx` returned). OyNIx calls its functions when things happen, if it has them: `pageShown(item)`, `tabOpened(item)`, `menuOpened(item)`, `navigated(url)`. |

`global.zxx`'s `glass: { blur, tint, opacity }` is the material of every glass element in OyNIx: the toolbar and tab strip, fields, buttons, cards and menus. The tint is mixed into each element's colour, `opacity` is how much of the element covers what's behind it, and `blur` frosts what shows through menus and popovers.

`global.zxx` can also make the window itself see-through: `window: { transparent: true, opacity: 0.2, surfaces: 0.4 }`. `opacity` sets how much of the window's colour covers the desktop, and `surfaces` does the same for bars, cards and fields. OyNIx gives its windows an alpha channel, so the theme applies as soon as it's chosen. Blurring the desktop behind the window is up to the compositor: Hyprland and KWin can do it, and picom does with a blur method configured. The built-in **Liquid Glass** theme does all of this, with animations.

StonedGem's own window reads a power theme's tokens, panels, `global.zxx` settings and pictures. The QML parts are for OyNIx.

## Tokens

| Kind | Tokens |
|---|---|
| Colours (`#rrggbb`) | `background` `surface` `surface_alt` `text` `muted` `accent` `on_accent` `border` `danger` `shadow` |
| Numbers (px) | `radius` `border_width` `shadow_x` `shadow_y` `shadow_blur`, and `font_weight` |
| Others | `glass` (true: Liquid Glass panels; false: flat, opaque ones), `font_family` |

**Panels** (`toolbar` `tab` `card` `button` `field` `popup`) each take
`fill` `text` `border` `radius` `border_width` `shadow_x` `shadow_y`.

Parts are merged in the table's order (`tokens.json`, `style.rs`,
`theme.zsp`, `theme.zxx`, `iced.toml`), and later parts win. In dark mode
each part's `dark` overrides are applied next. Anything still missing gets
a default worked out from the tokens that are set, and low contrast (text
on background, muted text, text on the accent) is reported as a warning.
The theme programs run in the Z#++ and Zsharpend sandboxes, with a 500 ms
limit.

Every part is checked when a theme is installed: JSON, TOML and Rust are
parsed, the programs and shader are compiled, and the TSX is transpiled. A
broken theme is refused with the file and line.

In the window, the tokens become the palette:

| Token | Used for |
|---|---|
| `background` | the window |
| `surface` / `surface_alt` | containers and raised controls |
| `accent` / `on_accent` | primary buttons, focus rings and selection |
| `radius` | rounding, scaled against the default 16 px |
| `border_width` / `border` | every panel's edge |
| `shadow_x` / `shadow_y` / `shadow` | hard offset shadows on tabs, cards, buttons and menus |

## Using one

**Settings › Appearance › Style** lists the built-in and installed themes.
The **Install** field takes a `.hypertheme` file or folder. Installed
themes live in the profile's `hyperthemes/` folder.

## Theme Studio

**Open theme…** lists the built-in and installed themes, power themes included. A theme with a `graph.json` opens as its graph. Any other theme opens as a graph built from its resolved tokens. Parts the Studio doesn't generate (a power theme's QML and programs, its modules, headers and pictures) are kept and saved back with the theme.


**Settings › Appearance › Open Theme Studio** (or Theme Studio in the menu)
opens a full-page node editor. It's a built-in **power extension**: its page
(`extra.html`) talks to its **Zsharpend** script (`script.zxx`) through
`chrome.runtime.sendMessage`.

- **Nodes:**
  - Colour, HSL, Mix, Shade, Readable (black or white, whichever reads better on the colour wired in)
  - Number, Font
  - Shader: `.zsg` code, previewed by the shader renderer
  - Panel
  - Theme: one in light mode; an optional second one in dark mode overrides it
- **Wires:** drag from an output dot to an input of the same type. Click a wire to remove it. Wires are drawn on a canvas used as the board's background, because Servo paints SVG and canvas elements above later siblings.
- **Live preview:** a small browser drawn with the CSS the browser would generate. On every change, the script turns the graph into all the parts and the browser resolves them.
- **Parts:** open any generated file to read it. **Save** writes the theme to the profile. **Save and use** applies it at once.

The script gets these power-extension functions (see
`stonedgem_extensions::zscript::POWER_API`):

- `to_json`, `from_json`
- `zsg_preview(code, w, h)`
- `hypertheme_resolve(files, dark)`
- `hypertheme_save(name, files, base)`
- `hypertheme_apply(file)`
- `hypertheme_list()`

## Neubrutalism

| Start page | Settings, dark |
|---|---|
| ![Neubrutalism start page](screenshots/neubrutalism-start.png) | ![Neubrutalism settings in dark mode](screenshots/neubrutalism-settings-dark.png) |

The built-in **Neubrutalism** theme was made in Theme Studio:

- cream paper (`#fff4dc`) and ink (`#111111`) with pink, yellow and cyan;
- 3 px borders, 4 px hard shadows (6 px on cards and menus) and 6 px corners;
- heavy type and a dot-grid backdrop shader;
- a dark mode that keeps the loud buttons on zinc.

Its `graph.json` is the source. The other parts are what the Studio's
script generates from it, and `cargo test -p stonedgem-hypertheme --test
studio` checks that they still match. After changing the script or the
graph, run that command with `REGENERATE=1` to rewrite them.
