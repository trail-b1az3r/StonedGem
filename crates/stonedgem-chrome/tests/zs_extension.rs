//! An extension's Z#++ script reacts to a page load: it injects CSS, runs
//! JavaScript and notifies — only on the pages its handler matches.

use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use std::time::Duration;

use stonedgem_chrome::zs_hooks::{css_js, Action, Scripts};
use stonedgem_engine::{Engine, Load, Value};
use stonedgem_extensions::registry;

fn serve() -> String {
    let l = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = l.local_addr().unwrap();
    std::thread::spawn(move || {
        for s in l.incoming().flatten() {
            let mut r = BufReader::new(s.try_clone().unwrap());
            let mut line = String::new();
            loop {
                line.clear();
                if r.read_line(&mut line).is_err() || line == "\r\n" || line.is_empty() {
                    break;
                }
            }
            let body = "<!DOCTYPE html><title>Shop</title><p id=p>hello</p>";
            let mut s = s;
            let _ = write!(s, "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len());
        }
    });
    format!("http://{addr}/shop")
}

#[test]
fn script_runs_on_matching_page_loads() {
    let root = std::env::temp_dir().join(format!("stonedgem-zs-ext-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::env::set_var("XDG_DATA_HOME", root.join("data"));
    std::env::set_var("XDG_CONFIG_HOME", root.join("config"));
    let src = root.join("src/tidy");
    std::fs::create_dir_all(&src).unwrap();
    std::fs::write(src.join("oynix.json"), r#"{"name": "Tidy", "version": "1.0", "matches": []}"#).unwrap();
    std::fs::write(src.join("code.ts"), "export {};").unwrap();
    std::fs::write(src.join("content.css"), "").unwrap();
    std::fs::write(src.join("tidy.zsx"), "func colour_for(title: string) -> string\n").unwrap();
    std::fs::write(
        src.join("script.zsp"),
        r##"import "tidy.zsx"
func colour_for(title: string) -> string { return title == "Shop" ? "rgb(1, 2, 3)" : "black" }
on page_load "http://127.0.0.1*" (url, title) {
    let n = (store_get("visits") ?? 0) + 1
    store_set("visits", n)
    inject_css("#p { color: ${colour_for(title)} }")
    run_js("document.title = 'Tidied ' + document.title")
    notify("Tidy ran on ${title} (${n})")
}
"##,
    )
    .unwrap();

    let mut list = vec![];
    let e = registry::install(&src, &mut list).expect("install");
    assert!(e.has_script);
    registry::save(&list);
    let mut scripts = Scripts::load(&registry::load());
    assert_eq!(scripts.len(), 1, "{:?}", scripts.errors);

    // A broken script is refused at install, with the reason.
    let bad = root.join("src/bad");
    std::fs::create_dir_all(&bad).unwrap();
    for (f, t) in [("oynix.json", r#"{"name": "Bad", "matches": []}"#), ("code.ts", "export {};"), ("content.css", ""), ("script.zsp", "on page_load { let = }\n")] {
        std::fs::write(bad.join(f), t).unwrap();
    }
    assert!(registry::install(&bad, &mut list).unwrap_err().contains("script.zsp"));

    let engine = Engine::headless(400, 300).expect("software rendering");
    let url = serve();
    let page = engine.open(&url, &[], &[]).unwrap();
    assert!(engine.wait(Duration::from_secs(30), || page.load() == Load::Complete));

    let actions = scripts.page_loaded(&url, "Shop");
    assert_eq!(actions.len(), 3, "{actions:?}");
    for (_, a) in &actions {
        let js = match a {
            Action::Css(c) => css_js(c),
            Action::RunJs(j) => j.clone(),
            Action::Notify(t) => {
                assert_eq!(t, "Tidy ran on Shop (1)");
                continue;
            }
            Action::PanelSet(..) | Action::ApplyHypertheme(_) => unreachable!("not in this script"),
        };
        page.eval(&engine, &js, Duration::from_secs(5)).unwrap();
    }
    let t = Duration::from_secs(5);
    assert_eq!(page.eval(&engine, "getComputedStyle(document.getElementById('p')).color", t), Ok(Value::String("rgb(1, 2, 3)".into())));
    assert_eq!(page.eval(&engine, "document.title", t), Ok(Value::String("Tidied Shop".into())));

    // Its saved value persists between page loads; other sites don't match.
    let again = scripts.page_loaded(&url, "Shop");
    assert!(again.iter().any(|(_, a)| a == &Action::Notify("Tidy ran on Shop (2)".into())));
    assert!(scripts.page_loaded("https://example.org/", "Elsewhere").is_empty());
}
