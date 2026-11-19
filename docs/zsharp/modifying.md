# Modifying Z#++

Z#++ is about 4,800 lines of Rust in `crates/stonedgem-zsharp`, with no unsafe code. This page explains how it's put together and walks through the four changes people make most: a new built-in, a new canvas method, new syntax, and a new GPU function.

## How a program runs

```text
source ─lexer.rs─▶ tokens ─parser.rs─▶ syntax tree (ast.rs)
                                         │
                  program.rs: imports, header checks
                                         │
            ┌────────────────────────────┴───────────────┐
        interp.rs                                    zsg.rs
        + stdlib.rs, value.rs, canvas.rs             .zsg files: type-check,
        walks the tree; built-ins, values,           WGSL (checked by naga),
        host objects, limits                         CPU executor
```

| File | What's in it |
|---|---|
| `lexer.rs` | Source text to `Token`s (`Tok::Int`, `Str` with interpolation parts, `Color`, `Ident`, `Op`, `Newline`). Newlines are tokens; `;` lexes as one. Operators are matched longest first from `OPS`. |
| `ast.rs` | The syntax tree: `Expr`, `Stmt`, `Item` (top-level things), `Kind` (program, header or GPU file). |
| `parser.rs` | A recursive-descent parser. `KEYWORDS` lists reserved words. Expression precedence is a chain of functions, loosest first: `expr` (pipe) → `ternary` → `coalesce` → `or` → `and` → `equality` → `compare` → `range` → `additive` → `multiplicative` → `power` → `unary` → `postfix` → `primary`. `Kind` decides what a file may contain (a header can't have bodies; `kernel` belongs in `.zsg`). |
| `program.rs` | Loads an entry file and its imports through the host's `read` function, checks headers against definitions, and hands everything to the interpreter. |
| `interp.rs` | The tree-walking interpreter: scopes (`Env`), `exec` for statements, `eval` for expressions, `binary` for operators, calls and methods, events (`emit`, `emit_values`), and the sandbox (`tick`, `Limits`). Control flow travels as `Ctrl` (`Return`, `Break`, `Continue`, `Throw`, and `Fatal` for limits, which `try` can't catch). |
| `stdlib.rs` | Built-in functions (`builtin`), the list of globals (`GLOBALS`), Z#'s `ZS.*` namespaces (`install`), and methods (`method`: every lower-case built-in is a method on its first argument). |
| `value.rs` | `Value`, the `HostObject` trait, structs, closures, scopes. |
| `canvas.rs` | `Canvas`, a host object drawing with tiny-skia, plus the bitmap font. |
| `zsg.rs` | GPU code: `Module::parse` type-checks, `to_wgsl` / `validate` compile, `run_kernel` / `shade` execute on the CPU (`Cpu`), and `ShaderRef` is the host object a program sees. |
| `bin/zspp.rs` | The command-line tool. |

Tests live in each file (`#[cfg(test)]`) and in `tests/language.rs`, which runs whole programs.

## Add a built-in function

Say we want `count(list, value)`: how many times `value` appears.

**1. Implement it** in `stdlib.rs`, in `builtin`, next to its relatives:

```rust
"count" => f(name, |_, a| {
    let xs = list(&a, 0, "count")?;
    Ok(Value::Int(xs.iter().filter(|x| x.equals(&arg(&a, 1))).count() as i64))
}),
```

The helpers `arg`, `num`, `int`, `list` and `text` read arguments and produce errors that name the function (“count: argument 1 must be a list, got int”). A function that can make something big — a list, a string — checks `it.check_len(n)` first so the sandbox's size limit holds.

**2. Make it a global** by adding `"count"` to `GLOBALS`. Lower-case built-ins are methods too, so `xs.count(3)` and `xs |> count(3)` work without further changes.

**3. Test it** in `tests/language.rs`:

```rust
#[test]
fn count_counts() {
    assert_eq!(run("func Main() { Printl([1, 2, 1].count(1)) }"), "2\n");
}
```

**4. Document it** in `docs/zsharp/reference.md`. The docs' tests fail if any name in `GLOBALS` is missing from the reference, so this step can't be forgotten.

## Add a canvas method

Canvas methods are a `match` in `Canvas::call` (`canvas.rs`). An ellipse, using tiny-skia's path builder:

```rust
"ellipse" => {
    let (cx, cy, rx, ry) = (n(&a, 0, m)?, n(&a, 1, m)?, n(&a, 2, m)?, n(&a, 3, m)?);
    let rect = Rect::from_xywh(cx - rx, cy - ry, rx * 2.0, ry * 2.0);
    self.fill_path(rect.and_then(PathBuilder::from_oval), color(&a, 4, m)?);
}
```

Then add a row to the table in `docs/zsharp/canvas.md` and a test that reads a pixel back with `c.pixel(x, y)`.

New kinds of host object follow the same pattern: implement `HostObject` (`type_name`, `field`, `call`, `as_any`) and hand scripts a `Value::Object`.

## Add syntax

Syntax touches more files. As an example, `unless cond { … }`, which runs its block when the condition is false.

**1. Reserve the word.** Add `"unless"` to `KEYWORDS` in `parser.rs` (and to the docs highlighter's list in `crates/stonedgem-zsharp-docs/src/highlight.rs`). Keywords are lexed as identifiers, so the lexer doesn't change. A new operator would go into `OPS` in `lexer.rs` instead, longest first.

**2. Parse it.** Statements are recognised in `Parser::stmt`. The simplest way to add `unless` is to turn it into an `if` with the condition negated, so nothing after the parser needs to know about it:

```rust
if self.eat_kw("unless") {
    let cond = self.expr(true)?;           // `true`: a `{` here starts the block, not a struct
    let then = self.block()?;
    self.end_stmt()?;
    return Ok(Stmt::If { cond: Expr::Unary("!", Box::new(cond)), then, els: None, line });
}
```

**3. Or give it its own node.** When a construct can't be rewritten into existing ones, add a variant to `Stmt` or `Expr` in `ast.rs`, run it in `Interp::exec` / `Interp::eval`, and decide what `.zsg` does with it: either emit WGSL and run it in `Cpu` (`zsg.rs`), or return a clear error like `throw`/`try` do (“aren't available on the GPU”). The compiler will point out every `match` that needs the new case.

**4. Test it** in `tests/language.rs`, including the error a wrong use gives, and **document it** on the right page of these docs with a `zsp run` example.

## Add a GPU function

GPU built-ins live in `zsg.rs`, in three places that must agree:

1. **Its type**, in `Module::builtin_ty`: what it returns for the argument types it's given. One-argument maths functions share an arm that returns the argument's type (with `int` widened to `float`).
2. **Its WGSL**, in `Module::emit`. A function with the same name in WGSL needs nothing here: the last arm writes `name(args)`, converting `int` arguments to `f32`. Only functions that differ (like `rgba` or `len`) have their own arm.
3. **Its CPU version**, in `Cpu::call`, on scalars and vectors: `map1` applies a one-argument function per component, `zip3` a three-argument one.

For `saturate(x)` (clamp to 0..1), which WGSL has built in:

```rust
// Module::builtin_ty — add it to the one-argument arm
"sin" | "cos" | /* … */ | "round" | "saturate" => fl(first),

// Cpu::call
"saturate" => map1(&arg(0), |x| x.clamp(0.0, 1.0))?,
```

`validate()` runs naga on the WGSL, so a mistake in step 2 is caught by the tests in `zsg.rs`; add one that runs the function on the CPU and compares the result, as `cpu_kernel_and_fragment` does. Then list it in the table on the [GPU code](gpu.md) page.

## Error messages

Errors are part of the language. They start with the line (`line 3: …`), say what was wrong in the reader's terms, and when there's an obvious fix, say it: “`global` goes before a declaration (`global int x = 1`)”, “in .zsg, `for` loops over a range (`for i in 0..n`)”. Avoid internal names (token kinds, Rust types) in messages.

## Before sending a change

```sh
cargo test -p stonedgem-zsharp           # the language
cargo test -p stonedgem-zsharp-docs      # every example in these docs still runs
cargo clippy --all-targets -- -D warnings
cargo run -p stonedgem-zsharp-docs       # build the site and look at the pages you changed
```
