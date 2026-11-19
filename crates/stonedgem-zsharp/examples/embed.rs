//! Embedding Z#++ in a Rust program: load a script with a host function,
//! run it, call into it, and emit an event. Shown in docs/zsharp/embedding.md.
//!
//! cargo run -p stonedgem-zsharp --example embed

use std::collections::{BTreeMap, HashMap};

use stonedgem_zsharp::{Limits, Program, Value};

const MAIN: &str = r#"
import "shop.zsx"

func total(items) {
    return items.map(|i| price_of(i)).sum()
}

on order (items) {
    Printl("order of ${len(items)}: ${total(items)}")
    return total(items) > 10
}
"#;

const HEADER: &str = "func total(items: list) -> int\nextern func price_of(item: string) -> int\n";

fn main() -> Result<(), String> {
    // Scripts read files only through this function.
    let files: HashMap<&str, &str> = [("main.zsp", MAIN), ("shop.zsx", HEADER)].into();
    let read = |path: &str| files.get(path).map(|s| s.to_string());

    // `setup` runs before the script loads: register what it may call.
    let prices: BTreeMap<&str, i64> = [("tea", 3), ("cake", 9)].into();
    let mut program = Program::load("main.zsp", &read, |it| {
        it.limits = Limits { steps: 100_000, ..Limits::default() };
        it.register("price_of", move |_, args| {
            let item = args.first().map(|v| v.to_string()).unwrap_or_default();
            prices.get(item.as_str()).map(|p| Value::Int(*p)).ok_or(format!("no price for {item}"))
        });
    })?;

    // Call a script function directly…
    let items = Value::list(vec![Value::str("tea"), Value::str("cake")]);
    let total = program.interp.call_global("total", vec![items.clone()])?;
    println!("total() returned {}", total.map(|v| v.to_string()).unwrap_or_default());

    // …or emit an event to its `on` handlers and read what they returned.
    let answers = program.interp.emit_values("order", None, vec![items])?;
    println!("handlers answered {answers:?}", answers = answers.iter().map(|v| v.to_string()).collect::<Vec<_>>());
    print!("script printed: {}", program.output());
    Ok(())
}
