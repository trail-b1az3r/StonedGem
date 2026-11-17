//! Site-compatibility survey for the engine spike.
//!
//! ```sh
//! cargo run -p stonedgem-engine --example sitecheck -- sites.txt > results.jsonl
//! ```
//!
//! `sites.txt` has one URL per line (`#` comments). For each site it loads
//! the page in a fresh headless page, waits until it has loaded (or 30 s),
//! and prints one JSON line: url, whether it loaded, seconds taken, title,
//! final URL, how much text and how many links/images the page ended up
//! with, how many requests were blocked, and console messages. Score each
//! site works / usable / broken by comparing with OyNIx; see
//! docs/ENGINE-SPIKE.md.

use std::time::{Duration, Instant};

use stonedgem_engine::{Engine, Load, Value};

fn esc(s: &str) -> String {
    let mut o = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => o.push_str("\\\""),
            '\\' => o.push_str("\\\\"),
            '\n' => o.push_str("\\n"),
            c if (c as u32) < 0x20 => o.push_str(&format!("\\u{:04x}", c as u32)),
            c => o.push(c),
        }
    }
    o.push('"');
    o
}

fn num(v: Result<Value, String>) -> String {
    match v {
        Ok(Value::Number(n)) => format!("{n}"),
        _ => "null".into(),
    }
}

fn main() {
    let path = std::env::args().nth(1).unwrap_or_else(|| {
        eprintln!("usage: sitecheck SITES.txt");
        std::process::exit(2)
    });
    let list = std::fs::read_to_string(&path).unwrap_or_else(|e| {
        eprintln!("{path}: {e}");
        std::process::exit(2)
    });
    stonedgem_blocker::blocker::init(vec![]);
    let engine = Engine::headless(1280, 800).expect("software rendering (needs Mesa's EGL)");
    for url in list.lines().map(str::trim).filter(|l| !l.is_empty() && !l.starts_with('#')) {
        let start = Instant::now();
        let page = match engine.open(url, &[], &[]) {
            Ok(p) => p,
            Err(e) => {
                println!("{{\"url\":{},\"error\":{}}}", esc(url), esc(&e));
                continue;
            }
        };
        let loaded = engine.wait(Duration::from_secs(30), || page.load() == Load::Complete);
        let secs = start.elapsed().as_secs_f64();
        let t = Duration::from_secs(5);
        let text = num(page.eval(&engine, "document.body ? document.body.innerText.length : 0", t));
        let links = num(page.eval(&engine, "document.links.length", t));
        let images = num(page.eval(&engine, "document.images.length", t));
        let console: Vec<String> = page.console().iter().take(20).map(|m| esc(m)).collect();
        println!(
            "{{\"url\":{},\"loaded\":{loaded},\"seconds\":{secs:.1},\"title\":{},\"finalUrl\":{},\"textChars\":{text},\"links\":{links},\"images\":{images},\"blocked\":{},\"console\":[{}]}}",
            esc(url),
            esc(&page.title().unwrap_or_default()),
            esc(&page.url().unwrap_or_default()),
            page.blocked().len(),
            console.join(",")
        );
    }
}
