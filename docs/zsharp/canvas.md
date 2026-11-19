# Drawing

`Canvas(width, height)` makes an image to draw on, up to 4096 × 4096 pixels. Shapes are anti-aliased; colours with alpha blend over what's below. The host can read the pixels back as RGBA or PNG; `zspp run --png` saves the canvas a program keeps in a global named `canvas` or `screen`.

```zsp run
func Main() {
    global let canvas = Canvas(520, 180)
    let c = canvas
    c.clear(#fbfaf7)
    c.rect(20, 20, 120, 60, #e3e0d8)
    c.stroke_rect(20, 100, 120, 60, 3, #1c1d1f)
    c.round_rect(160, 20, 120, 140, 24, #1d5bb8)
    c.circle(350, 60, 40, with_alpha(#a3322a, 0.8))
    c.stroke_circle(380, 90, 40, 4, #2f6b3a)
    c.line(300, 160, 500, 20, 2, #8a5a00)
    c.polygon([NVec2(420, 160), NVec2(500, 160), NVec2(460, 110)], #6b4fa0)
    c.polyline([NVec2(20, 170), NVec2(80, 168), NVec2(140, 172)], 2, #5b5f66)
    c.text(176, 40, "TEXT", 14, #ffffff)
    Printl("pixel at (60, 50) is ${c.pixel(60, 50)}")
}
```

## Methods

Positions and sizes are in pixels from the top-left; colours are colour values.

| Method | Draws |
|---|---|
| `clear(colour)` | fills everything |
| `rect(x, y, w, h, colour)` | a filled rectangle |
| `stroke_rect(x, y, w, h, width, colour)` | a rectangle's outline |
| `round_rect(x, y, w, h, radius, colour)` | a filled rectangle with rounded corners |
| `circle(cx, cy, r, colour)` | a filled circle |
| `stroke_circle(cx, cy, r, width, colour)` | a circle's outline |
| `line(x1, y1, x2, y2, width, colour)` | a line |
| `polygon(points, colour)` | a filled shape through `points` (`Vec2`s or `[x, y]` lists) |
| `polyline(points, width, colour)` | an open line through `points` |
| `text(x, y, string, size, colour)` | text in the built-in 5×7 pixel font; `(x, y)` is its top-left and `size` its height, rounded to a multiple of 7 |
| `shade(fragment, x, y, w, h, uniforms)` | fills the area with a `.zsg` fragment shader — see [GPU code](gpu.md) |
| `pixel(x, y)` | reads one pixel back, as a colour |

Fields: `width`, `height`.

> **Note** The built-in font covers letters (drawn in capitals), digits and common punctuation. It's meant for labels on generated images, not for typesetting.

## Drawing with code

Everything else in the language works on a canvas: loops, functions, colours mixed per step.

```zsp run
func Main() {
    global let canvas = Canvas(520, 200)
    canvas.clear(#10141c)
    let rings = 18
    for i in 0..rings {
        let t = i / (rings - 1.0)
        let r = 90 - i * 4.5
        let colour = mix(#2f7cf6, #e8b45c, t)
        canvas.stroke_circle(260 + 80 * cos(t * TAU), 100 + 20 * sin(t * TAU), r, 2, with_alpha(colour, 0.85))
    }
}
```

## Z#'s graphics

`ZS.Graphics.Init(title, width, height)` from Z# creates a canvas named `screen` and tells the host the program wants a window; the host then calls `Start()` once and `Update(dt)` every frame. See [Z# compatibility](z-sharp.md).
