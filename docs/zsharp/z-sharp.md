# Z# compatibility

Z#++ began as [Z#](https://github.com/sam-astro/Z-Sharp) by Sam-astro (MIT licence) and keeps running Z# programs as they are. This program is Z#'s own README example:

```zsp run
// Comments are indicated by two forward slashes
func Main()
{
    int i = 0
    string s = "r"

    i += 2
    i -= 1
    i /= 3
    i *= 2

    while i < 10
    {
        i += 1
    }

    if s == "r"
    {
        Printl(s + " is r")
    }

    int functionNumber = ExampleFunction("A", s)
    ExampleFunction(1, 3)

    GlobalFunction()
    Printl(x)
    Printl(y)
}

func ExampleFunction(inputA, inputB)
{
    Printl("In A is: " + inputA)
    Printl("In B is: " + inputB)
    return 4
}

func GlobalFunction()
{
    global int x = 12
    global string y = "Y String"
}
```

## What carries over

| Z# | In Z#++ |
|---|---|
| `func Main()` as the entry point | the same |
| `int i = 0`, `string s`, `float f`, `bool b`, `Vec2 v` | typed declarations; `let`, `var` and `const` also work |
| `global int x = 12` inside a function | the same |
| statements end at line ends; `while` / `if` without parentheses | the same; `;` is also allowed |
| `Printl`, `Print` | the same, plus lower-case `println`, `print` |
| `ZS.Math.Sin`, `Cos`, `Tan`, `Sqrt`, `Abs`, `Pow`, `Round`, `Floor`, `Ceil`, `Clamp`, `Lerp`, `Random` | the same functions, also as `sin`, `cos`, … |
| `NVec2(x, y)`, `Vec2` with `.x`, `.y`, `+`, `-`, `*` | the same |
| `ZS.Graphics.Init(title, w, h)`, `Start()`, `Update(dt)` | gives the program a `screen` canvas; the host calls `Start` and `Update` |
| `ZS.System.Command(…)` | exists, but refuses to run anything ([the sandbox](sandbox.md)) |

## What's different

- Z#++ is a separate implementation, in Rust, not a fork of the C++ interpreter.
- Programs run in [the sandbox](sandbox.md), with limits.
- Everything else in these docs is added on top: strings with `${…}`, colours, lists and maps, `match`, lambdas and the pipe, structs with methods, errors, events, headers, the canvas and GPU code.
