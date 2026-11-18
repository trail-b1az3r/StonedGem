//! Installed .oynix extensions run on the pages they match, and only there.

use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use std::time::Duration;

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
            let body = "<!DOCTYPE html><title>Ext</title><p id=p>hello</p>";
            let mut s = s;
            let _ = write!(s, "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len());
        }
    });
    format!("http://{addr}/page")
}

fn make_ext(dir: &std::path::Path, name: &str, matches: &str, flag: &str) {
    std::fs::create_dir_all(dir).unwrap();
    std::fs::write(dir.join("oynix.json"), format!(r#"{{"name": "{name}", "version": "1.0", "matches": [{matches}]}}"#)).unwrap();
    std::fs::write(dir.join("code.ts"), format!("(window as any).{flag} = document.getElementById('p') ? 'ran-after-dom' : 'ran-early';")).unwrap();
    std::fs::write(dir.join("content.css"), "p { color: rgb(1, 2, 3); }").unwrap();
}

#[test]
fn matching_extensions_run_and_others_dont() {
    let root = std::env::temp_dir().join(format!("stonedgem-ext-pages-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::env::set_var("XDG_DATA_HOME", root.join("data"));
    std::env::set_var("XDG_CONFIG_HOME", root.join("config"));

    make_ext(&root.join("src/here"), "Here", r#""http://127.0.0.1/*""#, "sgHere");
    make_ext(&root.join("src/there"), "There", r#""*://example.org/*""#, "sgThere");
    let mut list = vec![];
    registry::install(&root.join("src/here"), &mut list).expect("install Here");
    registry::install(&root.join("src/there"), &mut list).expect("install There");
    registry::save(&list);
    let scripts = registry::page_scripts(&registry::load());
    assert_eq!(scripts.len(), 2);

    let engine = Engine::headless(400, 300).expect("software rendering");
    let page = engine.open(&serve(), &scripts, &[]).unwrap();
    assert!(engine.wait(Duration::from_secs(30), || page.load() == Load::Complete));
    let t = Duration::from_secs(5);
    assert_eq!(page.eval(&engine, "window.sgHere", t), Ok(Value::String("ran-after-dom".into())), "matching extension, at document_end");
    assert_eq!(page.eval(&engine, "typeof window.sgThere", t), Ok(Value::String("undefined".into())), "other site's extension must not run");
    assert_eq!(page.eval(&engine, "getComputedStyle(document.getElementById('p')).color", t), Ok(Value::String("rgb(1, 2, 3)".into())), "content.css applies");

    // A turned-off extension contributes nothing.
    let mut list = registry::load();
    list.iter_mut().for_each(|e| e.enabled = false);
    assert!(registry::page_scripts(&list).is_empty());
}
