# The basics

A Z#++ file is a list of statements and declarations. Statements end at the end of a line; `;` also ends one, so two can share a line. A line continues when it clearly isn't finished: after an operator, a comma or an opening bracket.

## Values

| Type | Examples | Notes |
|---|---|---|
| `int` | `42`, `-7`, `0xFF`, `1_000_000` | 64-bit |
| `float` | `3.14`, `2.0`, `1e-3`, `2f` | 64-bit |
| `bool` | `true`, `false` | |
| `string` | `"text"`, `'text'`, `"hi ${name}"` | Unicode; `${…}` interpolates |
| `color` | `#f80`, `#2f7cf6`, `#2f7cf680` | RGBA, 8 bits per channel |
| `list` | `[1, 2, 3]` | grows with `push` |
| `map` | `{ ada: 36, "two words": 2 }` | string keys, kept sorted |
| range | `0..10`, `0..=10` | `..` leaves the end out, `..=` includes it |
| `null` | `null` | nothing |

Structs, functions and host objects (a canvas, a shader) are values too. `type(x)` names the type of any value.

```zsp run
func Main() {
    for v in [42, 2.5, true, "hi", #ff8800, [1, 2], { a: 1 }, 0..3, null] {
        Printl("${v} is ${type(v)}")
    }
}
```

## Declarations

`let` and `var` declare a variable; `const` declares one that can't be reassigned. A type is optional and written after the name. Z#'s style, the type first, works as well.

```zsp run
const LIMIT: int = 10
let name = "Ada"
var count = 0
count += 3

int legacy = 5          // Z# style
string greeting = "hi"

func Main() {
    Printl(name, count, LIMIT, legacy, greeting)
}
```

Reassigning a constant is an error when it happens:

```zsp error
const LIMIT = 10
func Main() {
    LIMIT = 11
}
```

`global` before a declaration inside a function puts the variable in the program's global scope, so other functions (and the host) can see it:

```zsp run
func Setup() {
    global let level = 3
}
func Main() {
    Setup()
    Printl("level ${level}")
}
```

## Strings

Strings use double or single quotes and can't span lines (write `\n`). `${…}` inside a string evaluates any expression.

```zsp run
func Main() {
    let items = ["tea", "toast"]
    Printl("You ordered ${len(items)} things: ${items.join(" and ")}.")
    Printl("Tab\tseparated, and a literal \${not interpolated}")
    Printl("Z#++".upper(), "  padded  ".trim(), "a,b,c".split(","))
}
```

Every [built-in](reference.md#strings) also works as a method on its first argument, so `upper(s)` and `s.upper()` are the same call.

## Colours

Colours are values, not strings: `#rgb`, `#rrggbb` or `#rrggbbaa`, or built with `rgb`, `rgba`, `hsl`, `mix` and `with_alpha`. Their channels are fields.

```zsp run
func Main() {
    let accent = #2f7cf6
    let soft = mix(accent, #ffffff, 0.6)
    Printl(accent, soft, with_alpha(accent, 0.5))
    Printl(hsl(30, 1, 0.5), rgb(255, 128, 0))
    Printl("red channel: ${accent.r}")
}
```

## Operators

From loosest to tightest binding:

| Operators | Meaning |
|---|---|
| `\|>` | pipe: `x \|> f(a)` is `f(x, a)` |
| `c ? a : b` | conditional |
| `??` | `a ?? b` is `a` unless it's `null` |
| `or`, `\|\|` | either |
| `and`, `&&` | both |
| `==`, `!=` | equal, not equal (by value) |
| `<`, `<=`, `>`, `>=`, `in` | compare; `in` tests membership in a list, map or string |
| `..`, `..=` | ranges |
| `+`, `-` | add (also joins strings and lists), subtract |
| `*`, `/`, `%` | multiply (a string times an int repeats it), divide, remainder |
| `**` | power (right to left: `2 ** 3 ** 2` is `2 ** 9`) |
| `-x`, `not x`, `!x` | negate |
| `f(x)`, `a[i]`, `a.b`, `a?.b` | call, index, field, field unless `null` |

Dividing two ints gives an int (`7 / 2` is `3`); make either side a float for a fractional result. `in` on strings is case-sensitive.

Assignment is a statement: `=`, `+=`, `-=`, `*=`, `/=`, `%=`, on a variable, a list or map element, or a field.

```zsp run
func Main() {
    let user = { name: "Ada", address: null }
    Printl(user.address?.city ?? "no city")
    Printl(3 in [1, 2, 3], "da" in "Ada", "ad" in "Ada", "name" in user)
    Printl(7 / 2, 7.0 / 2, 7 % 3, 2 ** 10)
    Printl([1, 2] + [3], "ab" * 3)
}
```

## Comments

`//` to the end of the line, and `/* … */` across lines.
