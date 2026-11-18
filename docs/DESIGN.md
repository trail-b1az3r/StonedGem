# StonedGem / OyNIx 5.4 design: the classic layout, in Liquid Glass

One look for the Iced window (`stonedgem-chrome`) and OyNIx 5.4's QML UI:
OyNIx's familiar layout (a full-width toolbar, the tab row, the page),
with Liquid Glass materials on the controls. No gradient backdrops.

| Source | What we take |
|---|---|
| OyNIx 5.3 | The **layout**: toolbar across the top (navigation, address field, actions), tabs below it, the page below that; start page, popovers and cards where they were. |
| Liquid Glass | The **material**: controls are glass — translucent, with a light rim and a bright specular line along the top edge. Glass is for controls only (never content, never glass on glass). Controls that aren't in use stay clear and turn to glass under the pointer; the address field and the selected tab are glass at rest. Tint only where colour means something (live downloads, blocked count, zoom ≠ 100 %, the checked state). |
| Material 3 | Colour **roles** from one accent (`stonedgem-theme`), the shape and type scales, state layers, filled/tonal buttons. |
| Apple design tips | Click targets ≥ 40 px, 8-point spacing, one large title per page, text contrast ≥ 4.5:1, labels for icon-only controls. |

## Tokens

- **Glass fill** (controls in use): surface-container-highest at 55 % (dark) / 62 % (light).
- **Strong glass** (legible at rest: address field, selected tab): 78 % / 86 %.
- **Rim:** white 14 % (dark) / outline-variant 65 % (light), 1 px.
- **Specular line:** white 20 % (dark) / 95 % (light), 1 px along the top edge.
- **Glass bar** (toolbar + tabs): surface-container at 88 % / 86 %, specular
  line on top, hairline (outline-variant 70 %) below.
- **Popups:** surface-container at 94–96 %, rim; menus keep a soft shadow.
- **Backdrop:** the plain surface colour. No gradients.
- **Spacing:** 4, 8, 12, 16, 24, 32. **Shape:** 12 (menu items), 16 (tabs),
  20 (popups), 24 (cards), full (capsules). **Type:** 34 bold (page title),
  20 semibold (section), 15 (body), 13 (secondary), 12 (labels).

### Dynamic refraction

Glass that floats over the page — menus, popovers, the right-click menu,
notifications — is real Liquid Glass: it shows the live page behind it,
frosted and tinted, bent inward near its rounded rim like a lens (with a
little colour dispersion), with a specular rim lit from the pointer's
position, so the highlight moves as the pointer does. Both apps use the
same model: OyNIx as a Qt Quick shader (`assets/shaders/liquidglass.frag`
there, over a live capture of the window), the Iced window on the CPU from
Servo's page frames (`stonedgem_chrome::glass`). The tint stays strong
enough for text on the glass to keep 4.5:1 contrast over busy pages.
Bars (toolbar, tabs) keep the glass material without refraction: nothing
is drawn behind them.

In the Iced window, shadows are kept to popups: Iced's CPU renderer
repaints drop shadows over neighbouring content when it redraws part of
the window, so glass there is defined by fill, rim and specular line.

## Layout (browser window)

```
─────────────────────────────── specular line ───────────────────────────────
 ←  →  ↻  ⌂   ( StonedGem │ address …                       ☆ )  125%  ⇩  ⋮
 ( Tab ● ✕ )  ( Tab ✕ )  ( Tab ✕ )  +
 [ find … ]  3 of 58  ↑ ↓                                              ✕
──────────────────────────────── hairline ──────────────────────────────────
 page (edge to edge)  ·  or History / Bookmarks / Downloads / Settings
```
