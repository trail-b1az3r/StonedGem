# Z#++

**Z#++** is StonedGem's scripting language: [Sam-astro's Z-Sharp](https://github.com/sam-astro/Z-Sharp)
(MIT) kept compatible — Z# programs run unchanged — and greatly extended,
sandboxed for a browser. It lives in `crates/stonedgem-zsharp`, written in
Rust (a reimplementation, not the C++ interpreter: the browser needs a
sandbox and a library API, which the original doesn't have).

| File | What it holds |
|---|---|
| `.zsp` (also Z#'s `.zs`) | **Programs**: functions, structs, statements, event handlers. |
| `.zsx` | **Headers**: declarations only — function signatures, structs, constants, `extern func`. Every function a header declares must be defined by the program with the same parameters and types, or provided by the host. |
| `.zsg` | **GPU code**: compute `kernel`s (general-purpose, not only rendering) and `fragment` shaders, with typed helper functions and constants. Compiled to WGSL (validated with naga) and runnable on the CPU. |

```sh
zspp run app.zsp --png out.png   # run Main(); save its canvas
zspp check ui.zsx                # parse/check any of the three
zspp wgsl effects.zsg            # print validated WGSL
```

## Z# compatibility

Everything in Z#'s README runs as written: `func Main()`, typed
declarations (`int i = 0`, `string s = "r"`, `Vec2 p = NVec2(1, 2)`),
`global int x = 12`, `+= -= *= /=`, `while`/`if` without parentheses,
statements ended by line breaks, `Printl`/`Print`, `ZS.Math.*`, and
`ZS.Graphics.Init(title, w, h)` (gives the program a `screen` canvas; the
host calls `Start()` and `Update(dt)`). `ZS.System.Command` exists but is
refused: scripts can't run programs.

## What Z#++ adds

```text
// Declarations: let/var, const, optional types
let name = "Ada"
const LIMIT: int = 10
var count = 0

// Strings with interpolation, colours as literals
Printl("Hello ${name}, ${count + 1} visit(s)")
let accent = #2f7cf6          // #rgb, #rrggbb, #rrggbbaa; rgb(), rgba(), hsl()

// Lists, maps, ranges, membership
let xs = [3, 1, 2]
let ages = { ada: 36, alan: 41 }
for i in 0..10 { }            // 0..=10 includes 10
for i, x in xs { }            // index and value
if "ada" in ages { }

// Functions: defaults, lambdas, the pipe
func greet(who, greeting = "Hi") { return "${greeting} ${who}" }
let total = xs |> filter(|x| x > 1) |> map(|x| x * 10) |> sum()
xs.map(|x| x * 2).join(", ")  // every built-in is also a method

// Structs with methods; Vec2-style structs add and scale
struct Point { x: float, y: float }
impl Point { func len(self) -> float { return sqrt(self.x ** 2 + self.y ** 2) } }
let p = Point { x: 3, y: 4 }  // or Point(3, 4)

// match: values, alternatives, ranges, guards
let size = match n { 0 => "none", 1..5 => "few", 5 | 6 => "some", _ if n < 0 => "?", _ => "many" }

// Null handling, conditions
let city = user?.address?.city ?? "unknown"
let label = n > 1 ? "items" : "item"
if ready and not busy { }

// Errors
try { risky() } catch e { Printl("failed: ${e}") }
throw "bad input"

// Events the host emits
on load { }
on page_load "*://*.example.com/*" (url, title) { }

// Modules
import "lib/ui.zsx"
import "effects.zsg"
```

Also: `break`, `continue`, `**`, `%`, `&&`/`and`, `||`/`or`, `!`/`not`,
`/* block comments */`, `;` as an optional statement separator, hex
`0xFF`, `1_000_000`.

**Built-ins** (functions and methods): `len str int float bool type range`
· `abs floor ceil round sqrt sin cos tan atan2 pow min max clamp lerp
random PI TAU` · `rgb rgba hsl mix with_alpha` · `push pop map filter reduce
sum sort reverse join first last slice zip` · `upper lower trim split
contains starts_with ends_with replace chars index_of repeat` · `keys values
has remove get` · `assert` · `Canvas`.

## 2D: `Canvas`

```text
let c = Canvas(480, 220)
c.clear(#10141c)
c.rect(x, y, w, h, colour)          c.stroke_rect(x, y, w, h, width, colour)
c.round_rect(x, y, w, h, r, colour)
c.circle(cx, cy, r, colour)         c.stroke_circle(cx, cy, r, width, colour)
c.line(x1, y1, x2, y2, width, colour)
c.polygon([p1, p2, p3], colour)     c.polyline(points, width, colour)
c.text(x, y, "Hello", size, colour) // built-in 5×7 bitmap font
c.shade(fragment, x, y, w, h, { time: 1.5 })   // fill with a .zsg shader
c.pixel(x, y)                       c.width   c.height
```

Anti-aliased (tiny-skia); the host can read it as RGBA or PNG.

## `.zsg`: GPU code

```text
const BANDS = 3
func band(y: float, c: float, w: float) -> float { return smoothstep(w, 0.0, abs(y - c)) }

fragment aurora(uv: vec2, uniform time: float) -> color {
    var c = vec3(0.04, 0.05, 0.09)
    for i in 0..BANDS { c += vec3(0.2, 0.5, 1.0) * band(uv.y, 0.4 + 0.1 * float(i), 0.1) }
    return vec4(c, 1.0)
}

kernel normalize(buf xs: [float], uniform lo: float, uniform hi: float) {
    xs[gid] = clamp((xs[gid] - lo) / (hi - lo), 0.0, 1.0)
}
```

Types `float int bool vec2 vec3 vec4/color`, buffers `[float]`/`[int]`;
swizzles (`.xy`, `.rgb`); `sin cos tan sqrt abs floor ceil fract exp log
sign round min max clamp mix step smoothstep pow atan2 length distance dot
normalize vec2 vec3 vec4 float int len`. A **kernel** runs once per element
of its first buffer, with the index in `gid`; a **fragment** gets `uv`
(0..1) and `resolution`. In a program, `normalize.run({ xs: [...] }, { lo: 0,
hi: 10 })` runs a kernel and returns the buffers; `canvas.shade(aurora, …)`
runs a fragment. Each module compiles to one WGSL module (each entry point
in its own bind group) that naga validates; the CPU executor runs the same
code where there's no GPU, and is what the tests compare against.

## Sandbox

Scripts reach only what the host registers. No files (imports go through
the host's reader, never above the program's folder), no shell, no
network. Limits per run: 20 million steps, 64 nested calls, 10 million
list items/characters, 5 seconds, canvases up to 4096×4096. Exceeding
one stops the script with a message; `try` can't catch it.

## In StonedGem

- **Extensions**: an `.oynix` package may include `script.zsp`. It's
  checked at install and runs on `page_load` with `page_url()`,
  `page_title()`, `inject_css(css)`, `run_js(code)`, `notify(text)`,
  `store_get(key)`, `store_set(key, value)`.
- **The window**: the start page's art is drawn by a Z#++ program and
  shader; **stonedgem://zsharp** is a playground (editor, Run, output,
  canvas preview).

Example: `examples/zsharp/dashboard.zsp` (imports `ui.zsx` and
`aurora.zsg`) → [screenshot](screenshots/zsharp-dashboard.png).
