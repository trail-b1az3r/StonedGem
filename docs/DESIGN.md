# StonedGem / OyNIx 5.4 design

One look for the Iced window (`stonedgem-chrome`) and OyNIx 5.4's QML UI.
It borrows from four references and keeps what works on a desktop browser:

| Source | What we take |
|---|---|
| Material 3 | Colour **roles** from one accent (`stonedgem-theme`), the shape scale, the type scale, state layers (hover 8 %, pressed 12 %), tonal buttons. |
| Liquid Glass | A **layer split**: content at the back, controls on a translucent panel floating above it, with a backdrop that has something to show through (a soft gradient), a hairline highlight and a soft shadow. Glass is for controls only — never content, never glass on glass. Tint only where colour means something (live downloads, blocked count, zoom ≠ 100 %). |
| Neumorphism | Used **sparingly**: the selected tab and cards are softly *raised* (shadow + highlight edge); the address field is softly *inset*. Never for low-contrast text or as the only sign of state. |
| Apple design tips | Click targets ≥ 40 px (toolbar buttons 40×40, rows ≥ 44), 8-point spacing, clear hierarchy (one large title per page), text contrast ≥ 4.5:1, labels on icon-only controls in tooltips/menus. |

## Tokens

- **Spacing:** 4, 8, 12, 16, 24, 32.
- **Shape:** 8 (small), 12 (menu items), 16 (fields inside cards), 20 (popups),
  22 (the floating panel), 24 (cards), full (capsules: tabs, address, buttons).
- **Type:** 34 bold (page title), 20 semibold (section), 15 (body), 13 (secondary),
  12 (labels, shortcuts).
- **Backdrop:** a linear gradient from the surface toward the primary (≈16 %)
  at the top-left and the tertiary (≈10 %) at the bottom-right.
- **Glass panel:** surface-container-high at ≈72 % (dark) / 74 % (light),
  1 px highlight edge (white 10 % dark / 70 % light), and a soft contact
  shadow in the gap below it.
- **Raised (neumorphic):** a lighter tone than its surroundings and a light edge.

- **Inset:** surface-container-lowest at 55 %, 1 px outline-variant edge.
- **Popups:** surface-container at 94 %, highlight edge, shadow 0 16 40 at 45 %.

In the Iced window, shadows are kept to popups (menus). Iced's CPU
renderer repaints drop shadows over neighbouring content when it redraws
part of the window, so the panel's shadow is a gradient strip, and cards
and the selected tab are raised by tone and edge. OyNIx (Qt Quick) uses a
real blurred shadow under its panel.

## Layout (browser window)

```
╭──────────────────── glass panel (8 px from the window edges) ─────────────────────╮
│  ( Tab ●)  ( Tab )  ( Tab )   +                                                    │
│  ←  →  ↻  ⌂   (  StonedGem │ address …                    ☆ )  125%  3 blocked  ⇩ ⋮ │
│  [ find … ]  3 of 58  ↑ ↓                                              ✕  (when open)│
╰────────────────────────────────────────────────────────────────────────────────────╯
 page (edge to edge)  ·  or History / Bookmarks / Downloads / Settings on the backdrop
```
