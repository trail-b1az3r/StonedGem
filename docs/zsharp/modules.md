# Files, imports and headers

A program can be split across files with `import`. Paths are relative to the importing file and can't leave the program's folder; the host reads the files, so a script never opens anything itself.

```text
import "lib/ui.zsx"      // a header
import "effects.zsg"     // GPU code
import "helpers.zsp"     // another program file
```

Imported program files share one global scope with the importer. Each file is loaded once even if several files import it.

## Headers

A `.zsx` header declares what a program provides: function signatures, structs and constants, with no bodies. When a program loads, every function a header declares must be defined by the program with the same parameters and types, or supplied by the host. That makes a header a contract: a host can publish one, and a script that doesn't keep its side won't load.

```zsx
// Header: what the card program promises to define.
struct Card { title: string, value: int }
const CARD_W = 150
func draw_card(c: Canvas, card: Card, x: int, y: int)
extern func host_clock() -> float
```

`extern func` marks a function the host provides rather than the program.

A header can't contain code:

```zsx error
func area(w: int, h: int) -> int { return w * h }
```

The checks report what doesn't match, for example:

```text
`area` is declared in shapes.zsx but never defined (and the host doesn't provide it)
`area` is declared in shapes.zsx with 1 parameter(s) but defined with 2
`area`: parameter `s` is Size in shapes.zsx but Rect in the definition
```

## A complete example

`examples/zsharp/` in the repository is a small program in three files: a header, GPU code, and the program that imports both.

```zsx include=examples/zsharp/ui.zsx
```

```zsg include=examples/zsharp/aurora.zsg
```

```zsp run include=examples/zsharp/dashboard.zsp
```

`aurora` and `normalize` come from the `.zsg` file: importing GPU code makes each kernel and fragment a global the program can use. See [GPU code](gpu.md).
