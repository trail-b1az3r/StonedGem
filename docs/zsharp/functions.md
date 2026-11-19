# Functions and structs

## Functions

`func` declares a function. Parameters can have types and default values; the return type follows `->`. Types are checked where they're written in [headers](modules.md#headers) and GPU code; in programs they document intent.

```zsp run
func greet(who: string, greeting = "Hello") -> string {
    return "${greeting}, ${who}"
}
func Main() {
    Printl(greet("Ada"))
    Printl(greet("Alan", "Hi"))
}
```

A function without `return` returns `null`.

## Lambdas

`|a, b| expression` is a short function; `func(a) { … }` is a longer one. Both close over the variables around them.

```zsp run
func Main() {
    let step = 3
    let add_step = |x| x + step
    let label = func(x) {
        if x > 5 { return "big ${x}" }
        return "small ${x}"
    }
    Printl([1, 4].map(add_step).map(label))
}
```

## The pipe

`x |> f(a, b)` calls `f(x, a, b)`. It reads left to right, which suits chains of list work:

```zsp run
func Main() {
    let scores = [72, 95, 58, 88, 64]
    let passed = scores
        |> filter(|s| s >= 60)
        |> sort()
        |> map(|s| "${s}%")
        |> join(", ")
    Printl(passed)
}
```

Method calls do the same: `scores.filter(…).sort()` — every built-in is also a method on its first argument.

## Structs

`struct` declares a record type with named fields. Make one with braces (`Point { x: 1, y: 2 }`) or by calling the type with the fields in order (`Point(1, 2)`).

`impl` adds methods; a method's first parameter is `self`.

```zsp run
struct Point { x: float, y: float }

impl Point {
    func len(self) -> float { return sqrt(self.x ** 2 + self.y ** 2) }
    func scaled(self, k) { return Point(self.x * k, self.y * k) }
}

func Main() {
    let p = Point { x: 3, y: 4 }
    Printl(p.len(), p.scaled(2).x)
    p.x = 6
    Printl(p)
}
```

Structs whose fields are all numbers add and subtract field by field, and multiply by a number, which is how Z#'s `Vec2` works:

```zsp run
func Main() {
    let a = NVec2(1, 2)
    let b = NVec2(10, 20)
    Printl(a + b, (b - a) * 0.5)
}
```

## Recursion

Functions can call themselves, up to 64 calls deep (see [the sandbox](sandbox.md)).

```zsp run
func fib(n) { return n < 2 ? n : fib(n - 1) + fib(n - 2) }
func Main() { Printl(range(12).map(fib)) }
```
