# Reference

Every built-in function, with the types it takes. Lower-case built-ins are also methods on their first argument (`len(xs)` is `xs.len()`) and work with the pipe (`xs |> len()`).

## Output

| Function | Does |
|---|---|
| `Printl(values…)`, `println(…)` | writes the values separated by spaces, then a line break |
| `Print(values…)`, `print(…)` | the same without the line break |

## Conversion

| Function | Returns |
|---|---|
| `str(x)`, `to_string(x)` | `x` as text, as `Printl` would write it |
| `int(x)`, `to_int(x)` | a whole number from a number or numeric text (fails on other text) |
| `float(x)`, `to_float(x)` | a float from a number or numeric text |
| `bool(x)` | whether `x` counts as true |
| `type(x)` | the name of `x`'s type: `"int"`, `"string"`, `"list"`, a struct's name… |
| `len(x)` | the length of a list, map, string (in characters) or range |
| `range(n)`, `range(a, b)` | `0..n` or `a..b` |
| `assert(cond, message?)` | fails with `message` when `cond` is false |

## Maths

| Function | Returns |
|---|---|
| `abs(x)` | the absolute value (an int stays an int) |
| `floor(x)`, `ceil(x)`, `round(x)` | an int |
| `sqrt(x)`, `pow(x, y)` | a float |
| `sin(x)`, `cos(x)`, `tan(x)`, `atan2(y, x)` | radians |
| `min(a, b, …)`, `max(a, b, …)` | the smallest or largest; with one list argument, of the list |
| `clamp(x, lo, hi)` | `x` limited to `lo..hi` |
| `lerp(a, b, t)` | `a + (b - a) * t` |
| `random()`, `random(lo, hi)` | a float in 0..1, or an int from `lo` to `hi` inclusive |
| `PI`, `TAU` | the constants (values, not functions) |

## Colours

| Function | Returns |
|---|---|
| `rgb(r, g, b)` | a colour from 0–255 channels |
| `rgba(r, g, b, a)` | with alpha: 0–1 as a float, or 0–255 as an int |
| `hsl(h, s, l)` | from hue (degrees), saturation and lightness (0–1) |
| `mix(a, b, t)` | colours or numbers blended: `t` of 0 is `a`, 1 is `b` |
| `with_alpha(colour, a)` | the colour with alpha `a` (0–1) |

A colour's channels are fields: `.r`, `.g`, `.b`, `.a` (0–255).

## Lists

| Function | Returns |
|---|---|
| `push(xs, x)` | `xs`, with `x` added at the end (changes `xs`) |
| `pop(xs)` | the last item, removed (or `null`) |
| `map(xs, f)` | `f` applied to each item |
| `filter(xs, f)` | the items for which `f` is true |
| `reduce(xs, f, start)` | `f(f(start, x0), x1)…` |
| `sum(xs)` | the total (an int if every item is) |
| `sort(xs, key?)` | a sorted copy; numbers and strings sort naturally; `key(x)` sorts by what it returns |
| `reverse(xs)` | a reversed copy (strings too) |
| `join(xs, sep?)` | the items as text, separated by `sep` |
| `first(xs)`, `last(xs)` | an end item (or `null`) |
| `slice(xs, start, end?)` | items from `start` up to `end` |
| `zip(a, b)` | pairs `[a0, b0], [a1, b1]…` |
| `contains(xs, x)`, `index_of(xs, x)` | whether `x` is there; where (or `-1`) |
| `remove(xs, i)` | the item at `i`, removed |

## Strings

| Function | Returns |
|---|---|
| `upper(s)`, `lower(s)`, `trim(s)` | changed copies |
| `split(s, sep?)` | a list of parts (default separator: a space) |
| `contains(s, t)`, `starts_with(s, t)`, `ends_with(s, t)` | true or false |
| `index_of(s, t)` | where `t` starts (or `-1`) |
| `replace(s, from, to)` | every `from` replaced |
| `chars(s)` | a list of one-character strings |
| `repeat(s, n)` | `s` `n` times (also `s * n`) |

## Maps

| Function | Returns |
|---|---|
| `keys(m)`, `values(m)` | lists, in key order (`keys` also works on structs) |
| `has(m, k)` | whether `k` is a key (also `k in m`) |
| `get(m, k, default?)` | the value, or `default` |
| `remove(m, k)` | the value, removed |

## Drawing and Z#

| Function | Returns |
|---|---|
| `Canvas(w, h)` | a [canvas](canvas.md) |
| `NVec2(x, y)`, `vec2(x, y)` | a `Vec2` struct |
| `ZS.Math.*`, `ZS.Graphics.Init`, `ZS.System.*` | Z#'s namespaces ([Z# compatibility](z-sharp.md)) |

## Keywords

`func let const var global if else while for in return break continue struct impl on import match try catch throw true false null and or not extern` — and, in `.zsg` files, `kernel fragment buf uniform`.

## Operator precedence

Loosest first: `|>` · `? :` · `??` · `or ||` · `and &&` · `== !=` · `< <= > >= in` · `.. ..=` · `+ -` · `* / %` · `**` · unary `- ! not` · calls, indexing, `.` and `?.`. See [The basics](basics.md#operators).
