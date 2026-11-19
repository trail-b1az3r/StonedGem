# Embedding in Rust

Z#++ is a Rust crate, `stonedgem-zsharp`, with no dependencies on the rest of StonedGem. A host loads a program, gives it functions and objects, runs it, and talks to it by calling its functions or emitting events.

```toml
[dependencies]
stonedgem-zsharp = { git = "https://github.com/trail-b1az3r/StonedGem" }
```

## A complete host

This is `crates/stonedgem-zsharp/examples/embed.rs` (`cargo run -p stonedgem-zsharp --example embed`): it loads a program and a header from memory, gives the program a `price_of` function, calls one of its functions and emits an event to it.

```rust include=crates/stonedgem-zsharp/examples/embed.rs
```

It prints:

```text
total() returned 12
handlers answered ["true"]
script printed: order of 2: 12
```

## Loading

| Call | Use |
|---|---|
| `Program::load(entry, read, setup)` | Load `entry` (a `.zsp`) and its imports. `read(path)` returns a file's text or `None`; it's the only way a script sees files. `setup(&mut Interp)` runs before the program does: register functions and globals there, so the program's top-level statements and header checks can see them. |
| `Program::from_source(src)` | One source string, no imports. |
| `program.run_main()` | Call `Main()` if the program has one. |
| `program.output()` | Everything `Printl`/`Print` wrote. Printing never reaches stdout by itself. |

Loading parses every file, checks headers against the definitions, and then runs the top-level statements in order.

## Giving scripts things

| Call | Use |
|---|---|
| `interp.register(name, f)` | A function scripts can call: `f(&mut Interp, Vec<Value>) -> Result<Value, String>`. An `Err` becomes a script error, which `try` can catch. |
| `interp.set_global(name, value)` | Any value as a global. |
| `Value::Object(Rc<dyn HostObject>)` | An object with fields and methods of your own (see below). |

### Host objects

Implement `HostObject` to give scripts an object they can read fields from and call methods on, and nothing else. Z#++'s own `Canvas` and GPU entry points are host objects.

```rust
use std::any::Any;
use stonedgem_zsharp::{value::HostObject, Interp, Value};

struct Page { url: String }

impl HostObject for Page {
    fn type_name(&self) -> &str { "Page" }
    fn field(&self, name: &str) -> Option<Value> {
        match name {
            "url" => Some(Value::str(&self.url)),
            _ => None,
        }
    }
    fn call(&self, _it: &mut Interp, method: &str, args: Vec<Value>) -> Result<Value, String> {
        match method {
            "has" => Ok(Value::Bool(self.url.contains(&args.first().map(|a| a.to_string()).unwrap_or_default()))),
            _ => Err(format!("Page has no method `{method}`")),
        }
    }
    fn as_any(&self) -> &dyn Any { self }
}
```

## Talking to a running program

| Call | Returns |
|---|---|
| `interp.call_global(name, args)` | `Ok(Some(value))` from a script function, `Ok(None)` if there's no such function |
| `interp.emit(event, subject, args)` | how many `on event` handlers ran; `subject` is matched against their filters |
| `interp.emit_values(event, subject, args)` | each handler's `return` value (`null` when it returned nothing) |
| `interp.global(name)` | a global's current value |

## Values

`Value` is an enum: `Null`, `Bool`, `Int(i64)`, `Float(f64)`, `Str`, `Color(u32)` (`0xRRGGBBAA`), `List`, `Map`, `Struct`, `Func`, `Native`, `Range`, `Object`. Build them with `Value::str(…)`, `Value::list(…)` and `Value::map(…)`; `value.to_string()` gives the text a script would print. Lists and maps are shared (`Rc<RefCell<…>>`), as they are in scripts.

## Reading a canvas

If a program draws, take its canvas from a global and read the pixels:

```rust
use stonedgem_zsharp::{canvas::Canvas, Value};

if let Some(Value::Object(o)) = program.interp.global("canvas") {
    if let Some(c) = o.as_any().downcast_ref::<Canvas>() {
        let rgba: Vec<u8> = c.rgba();          // c.width() × c.height() × 4, straight alpha
        let png: Vec<u8> = c.png()?;
    }
}
```
