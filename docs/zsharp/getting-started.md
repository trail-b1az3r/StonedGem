# Getting started

Z#++ lives in the StonedGem repository, in `crates/stonedgem-zsharp`. It has no dependencies outside Rust crates, so building it needs only a Rust toolchain.

## Install `zspp`

`zspp` runs and checks Z#++ files from a terminal:

```sh
git clone https://github.com/trail-b1az3r/StonedGem
cd StonedGem
cargo install --path crates/stonedgem-zsharp --bin zspp
```

Or run it in place with `cargo run -p stonedgem-zsharp --bin zspp -- …`.

## A first program

Save this as `hello.zsp`:

```zsp run
func Main() {
    let name = "world"
    Printl("Hello, ${name}!")
    for i in 1..=3 {
        Printl("${i} squared is ${i ** 2}")
    }
}
```

and run it:

```sh
zspp run hello.zsp
```

A program's entry point is `Main()`. Statements outside any function also run, in order, when the file loads, before `Main`.

## Drawing something

A program that keeps a canvas in a global called `canvas` (or `screen`) can save it as a PNG:

```zsp run
func Main() {
    global let canvas = Canvas(240, 120)
    canvas.clear(#f4f2ed)
    canvas.circle(70, 60, 40, #1d5bb8)
    canvas.stroke_circle(150, 60, 40, 6, #a3322a)
    canvas.text(16, 104, "two circles", 7, #1c1d1f)
}
```

```sh
zspp run circles.zsp --png circles.png
```

## Checking files

`zspp check` parses a file and checks it without running it: programs are loaded with their imports, headers are parsed, GPU code is type-checked and compiled.

```sh
zspp check app.zsp      # app.zsp: ok
zspp check ui.zsx
zspp check effects.zsg
zspp wgsl effects.zsg   # print the WGSL a .zsg compiles to
```

Errors name the file and line:

```zsp error
func Main() {
    let total = 0
    total += missing
}
```

## Next

- [The basics](basics.md): values, declarations, strings, colours, operators.
- [Drawing](canvas.md) and [GPU code](gpu.md).
