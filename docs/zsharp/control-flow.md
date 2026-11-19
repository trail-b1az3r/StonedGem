# Control flow

Conditions don't need parentheses, and bodies are always in braces. Z#'s style, with the brace on the next line, works too.

## if and else

```zsp run
func describe(n) {
    if n < 0 {
        return "negative"
    } else if n == 0 {
        return "zero"
    } else {
        return "positive"
    }
}
func Main() {
    Printl(describe(-2), describe(0), describe(5))
}
```

`null`, `false`, `0`, `0.0`, `""`, `[]` and `{}` count as false; everything else counts as true.

## while

```zsp run
func Main() {
    var n = 27
    var steps = 0
    while n != 1 {
        n = n % 2 == 0 ? n / 2 : 3 * n + 1
        steps += 1
    }
    Printl("27 reaches 1 after ${steps} steps")
}
```

## for

`for` walks a list, a range, a string's characters (`.chars()`) or a map's keys. With two names, the first is the index.

```zsp run
func Main() {
    for i in 0..3 { Print(i, "") }
    Printl("")
    for i, fruit in ["fig", "pear"] { Printl("${i}: ${fruit}") }
    let stock = { figs: 4, pears: 0 }
    for name in stock { Printl("${name} → ${stock[name]}") }
}
```

`break` leaves the loop; `continue` skips to the next turn.

## match

`match` compares a value against patterns, top to bottom, and is an expression: it has the value of the arm that matched. A pattern is a value, several values separated by `|`, a range, or `_` for anything. `if` after a pattern adds a condition.

```zsp run
func size(n) {
    return match n {
        0 => "none",
        1..5 => "a few",
        5 | 6 | 7 => "some",
        _ if n < 0 => "impossible",
        _ => "lots"
    }
}
func Main() {
    for n in [0, 3, 6, -1, 40] { Printl(n, size(n)) }
}
```

An arm can be a block: `"x" => { … }`.

## Errors

`throw` raises any value as an error; `try … catch` catches it. Errors from the language itself (dividing by zero, calling something that isn't a function) are caught the same way, as a message string.

```zsp run
func parse_age(s) {
    let n = int(s)
    if n < 0 { throw "an age can't be negative" }
    return n
}
func Main() {
    for s in ["36", "-4", "old"] {
        try {
            Printl("age", parse_age(s))
        } catch e {
            Printl("rejected ${s}: ${e}")
        }
    }
}
```

> **Note** Running out of the sandbox's limits (time, steps, call depth, size) stops the script and can't be caught. See [the sandbox](sandbox.md).
