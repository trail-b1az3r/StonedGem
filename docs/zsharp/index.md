# Z#++

Z#++ is the scripting language of StonedGem, the browser base under OyNIx. It's small, safe to run in a browser, and does two things scripting languages usually leave to other tools: it draws (a 2D canvas) and it runs GPU code (compute kernels and pixel shaders, compiled to WGSL).

```zsp run
func Main() {
    let c = Canvas(560, 140)
    c.clear(#10141c)
    for i in 0..14 {
        let h = 20 + 90 * abs(sin(i * 0.45))
        c.round_rect(24 + i * 38, 120 - h, 26, h, 6, hsl(210 + i * 6, 0.7, 0.62))
    }
    c.text(24, 16, "Z#++ draws this", 10, #e6e7e9)
    Printl("14 bars, tallest ${round(20 + 90 * max(range(14).map(|i| abs(sin(i * 0.45)))))}px")
    global let canvas = c
}
```

That program ran when this page was built, as did every example in these docs: what you see under each one is its real output.

## What it's for

- **Extensions.** A StonedGem or OyNIx extension can ship a `script.zsp` that reacts to pages loading, answers its content scripts' messages and drives its panel.
- **The browser itself.** The start page's banner is drawn by a Z#++ program and shader; OyNIx's colour themes are a Z#++ program.
- **Small programs.** `zspp run app.zsp --png out.png` runs anything from a terminal.

## Three kinds of file

| File | Holds |
|---|---|
| `.zsp` | A **program**: functions, structs, statements, event handlers. Z#'s `.zs` files load as programs too. |
| `.zsx` | A **header**: declarations only. Every function it declares must be defined by the program, or supplied by the host. |
| `.zsg` | **GPU code**: `kernel`s (general-purpose compute) and `fragment` shaders, with typed helpers. |

## Where it came from

Z#++ started from [Z#](https://github.com/sam-astro/Z-Sharp) by Sam-astro (MIT). Z# programs still run unchanged (see [Z# compatibility](z-sharp.md)). Z#++ is a separate implementation in Rust, written for a browser: everything a script can reach is given to it by the host, and every run has limits on time, steps and memory ([the sandbox](sandbox.md)).

## Reading these docs

- New to it: [Getting started](getting-started.md), then [The basics](basics.md).
- Putting it in your program: [Embedding in Rust](embedding.md).
- Changing the language: [Modifying Z#++](modifying.md).
- Looking something up: the [Reference](reference.md).
