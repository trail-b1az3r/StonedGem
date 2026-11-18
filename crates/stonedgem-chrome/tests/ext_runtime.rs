//! Extensions at run time, in Servo:
//!
//! * a power extension's `network.ts` runs in its background page: it adds
//!   blocker rules, sees the requests pages make, and fetches without
//!   cross-origin limits;
//! * a Chrome extension's content script uses `chrome.storage.local`,
//!   `chrome.runtime.sendMessage` (answered by its Z#++ script) and
//!   `chrome.tabs.create`;
//! * an extension's widget shows on the start page.
//!
//! (One test: Servo starts once per process.)

use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use std::time::{Duration, Instant};

use stonedgem_chrome::exthub::{self, Effect, ExtHub};
use stonedgem_chrome::zs_hooks::Scripts;
use stonedgem_engine::{Engine, Load, Value};
use stonedgem_extensions::registry;

fn serve() -> String {
    let l = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = l.local_addr().unwrap();
    std::thread::spawn(move || {
        for s in l.incoming().flatten() {
            let mut r = BufReader::new(s.try_clone().unwrap());
            let mut first = String::new();
            let _ = r.read_line(&mut first);
            let mut line = String::new();
            loop {
                line.clear();
                if r.read_line(&mut line).is_err() || line == "\r\n" || line.is_empty() {
                    break;
                }
            }
            let path = first.split_whitespace().nth(1).unwrap_or("/").to_string();
            let (ty, body) = match path.as_str() {
                "/data" => ("text/plain", "pong".to_string()),
                _ => ("text/html", format!("<!DOCTYPE html><title>Page {path}</title><body><p>hello</p></body>")),
            };
            let mut s = s;
            let _ = write!(s, "HTTP/1.1 200 OK\r\nContent-Type: {ty}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len());
        }
    });
    format!("http://{addr}")
}

fn write(dir: &std::path::Path, files: &[(&str, &str)]) {
    std::fs::create_dir_all(dir).unwrap();
    for (f, t) in files {
        std::fs::write(dir.join(f), t).unwrap();
    }
}

fn eval_str(engine: &Engine, page: &stonedgem_engine::Page, js: &str) -> String {
    match page.eval(engine, js, Duration::from_secs(2)) {
        Ok(Value::String(s)) => s,
        other => format!("{other:?}"),
    }
}

#[test]
fn network_ts_chrome_apis_and_widgets() {
    let root = std::env::temp_dir().join(format!("stonedgem-ext-runtime-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::env::set_var("XDG_DATA_HOME", root.join("data"));
    std::env::set_var("XDG_CONFIG_HOME", root.join("config"));
    std::env::set_var("XDG_CACHE_HOME", root.join("cache"));
    stonedgem_extensions::host::set(stonedgem_extensions::Host {
        name: "StonedGem",
        trusted_keys: "",
        check_qml: None,
        webchannel_js: Some(stonedgem_chrome::bridge::webchannel_shim),
    });
    stonedgem_blocker::blocker::init(vec![]);
    stonedgem_blocker::blocker::configure(true, true, true, false);
    let server = serve();

    // A power extension with network.ts (built in, so no signature needed).
    let net = root.join("src/net");
    let network_ts = format!(
        r#"declare var oynix: any;
const seen: string[] = [];
(window as any).seen = seen;
oynix.network.block(["||blocked.invalid^"]);
oynix.network.onRequest((r: any) => {{ seen.push(r.type + " " + r.url); }});
oynix.network.fetch("{server}/data").then((r: any) => r.text()).then((t: string) => {{ document.title = "fetched " + t; }}, (e: any) => {{ document.title = "failed " + e.message; }});
"#
    );
    write(
        &net,
        &[("oynix.json", r#"{"name": "Net", "version": "1.0", "power": true, "matches": []}"#), ("code.ts", "export {};"), ("content.css", ""), ("network.ts", &network_ts)],
    );
    // A Chrome extension whose Z#++ script answers its content script.
    let notes = root.join("src/notes");
    let content_js = format!(
        r#"chrome.storage.local.set({{ k: 5 }})
  .then(function () {{ return chrome.storage.local.get('k'); }})
  .then(function (v) {{ document.body.dataset.k = v.k; return chrome.runtime.sendMessage({{ ask: 'hi' }}); }})
  .then(function (r) {{ document.body.dataset.reply = r.answer + ' / ' + chrome.runtime.getManifest().name; }},
        function (e) {{ document.body.dataset.reply = 'error ' + e.message; }});
if (location.pathname === '/page') chrome.tabs.create({{ url: '{server}/other' }});
"#
    );
    write(
        &notes,
        &[
            (
                "manifest.json",
                r#"{"manifest_version": 3, "name": "Notes", "version": "1.0", "content_scripts": [{"matches": ["http://127.0.0.1/*"], "js": ["content.js"]}]}"#,
            ),
            ("content.js", &content_js),
            ("script.zsp", "on message (msg, sender) {\n    return { answer: \"${msg.ask} back\", from: sender.url }\n}\n"),
        ],
    );
    // An extension with a start-page widget.
    let widget = root.join("src/widget");
    write(
        &widget,
        &[
            ("oynix.json", r#"{"name": "Hello widget", "version": "1.0", "matches": []}"#),
            ("code.ts", "export {};"),
            ("content.css", ""),
            ("widget.tsx", "export default function W() { return <button>Hello</button>; }\n"),
        ],
    );

    let mut list = vec![];
    let n = registry::install_builtin(&net, &mut list).expect("install network.ts extension");
    assert!(n.power && n.has_network);
    let c = registry::install(&notes, &mut list).expect("install Chrome extension");
    assert!(c.has_script, "a WebExtension's script.zsp is picked up");
    let w = registry::install(&widget, &mut list).expect("install widget");
    assert!(w.has_widget);
    registry::save(&list);
    let list = registry::load();

    let engine = Engine::headless(800, 600).expect("software rendering");
    let mut hub = ExtHub::start(&engine, &list);
    assert!(hub.errors.is_empty(), "{:?}", hub.errors);
    assert_eq!(hub.background.len(), 1);
    let (scripts, content_api) = exthub::page_scripts(&list);
    assert!(content_api);
    hub.content_api = true;
    let mut zs = Scripts::load(&list);
    assert_eq!(zs.len(), 1, "{:?}", zs.errors);

    let page = engine.open(&format!("{server}/page"), &scripts, &[]).unwrap();
    let mut effects = vec![];
    let end = Instant::now() + Duration::from_secs(60);
    loop {
        engine.spin();
        std::thread::sleep(Duration::from_millis(20));
        effects.extend(hub.pump(&engine, &[(0, &page)], &mut zs, &list));
        let reply = eval_str(&engine, &page, "document.body ? (document.body.dataset.reply || '') : ''");
        let title = hub.background[0].page.title().unwrap_or_default();
        if (!reply.is_empty() && title.starts_with("fetched")) || Instant::now() > end {
            break;
        }
    }
    assert_eq!(page.load(), Load::Complete);

    // network.ts: fetch, block, onRequest.
    assert_eq!(hub.background[0].page.title().unwrap_or_default(), "fetched pong");
    assert!(effects.contains(&Effect::BlockerChanged), "{effects:?}");
    stonedgem_blocker::blocker::init(vec![]);
    let gen = stonedgem_blocker::blocker::generation();
    assert!(engine.wait(Duration::from_secs(10), || stonedgem_blocker::blocker::generation() > gen));
    assert!(stonedgem_blocker::blocker::should_block("http://blocked.invalid/x.js", &format!("{server}/page"), 3), "network.ts's rule blocks");
    // Loads after onRequest was called reach the listener.
    let second = engine.open(&format!("{server}/second"), &[], &[]).unwrap();
    assert!(engine.wait(Duration::from_secs(30), || second.load() == Load::Complete));
    let mut seen = String::new();
    for _ in 0..50 {
        hub.pump(&engine, &[], &mut zs, &list);
        engine.spin();
        seen = eval_str(&engine, &hub.background[0].page, "JSON.stringify(window.seen)");
        if seen.contains("/second") {
            break;
        }
    }
    assert!(seen.contains(&format!("main_frame {server}/second")), "{seen}");

    // chrome.* from the content script.
    assert_eq!(eval_str(&engine, &page, "document.body.dataset.k"), "5");
    assert_eq!(eval_str(&engine, &page, "document.body.dataset.reply"), "hi back / Notes");
    assert_eq!(stonedgem_chrome::bridge::storage_load(&c.id).get("k"), Some(&serde_json::json!(5)), "storage.local is saved in the profile");
    assert!(effects.contains(&Effect::OpenTab(format!("{server}/other"))), "{effects:?}");
    // The page itself can't read the extension's storage by queueing a
    // message under its id: without the shim's secret it isn't answered.
    let forge = format!(
        "window.__forged = 'none'; var f = window.__sgChrome[{id}]; window.__sgChrome[{id}] = function (req, v, e) {{ if (req === 'forged') window.__forged = JSON.stringify(v); return f(req, v, e); }};\
         window.__sgOut.push([{id}, 'chrome', JSON.stringify({{op: 'storage.get', req: 'forged', keys: null}})]); 'ok'",
        id = serde_json::to_string(&c.id).unwrap()
    );
    assert_eq!(eval_str(&engine, &page, &forge), "ok");
    assert!(hub.pump(&engine, &[(0, &page)], &mut zs, &list).is_empty());
    assert_eq!(eval_str(&engine, &page, "window.__forged"), "none");

    assert!(hub.stop(), "stopping drops network.ts's blocker rules");

    // The start page with the widget: a card whose frame draws the
    // widget's accent button.
    let start = stonedgem_chrome::start_page_with("StonedGem", &[(w.name.clone(), registry::widget_html(&w, stonedgem_chrome::WIDGET_THEME).unwrap())]);
    let sp = engine.open(&start, &[], &[]).unwrap();
    assert!(engine.wait(Duration::from_secs(30), || sp.load() == Load::Complete));
    assert_eq!(eval_str(&engine, &sp, "document.querySelector('.card h2').textContent"), "Hello widget");
    let accent = |f: &stonedgem_engine::Frame| {
        (0..f.height).step_by(2).any(|y| (0..f.width).step_by(2).any(|x| {
            let p = f.pixel(x, y);
            p[0].abs_diff(0x8f) < 6 && p[1].abs_diff(0xb4) < 6 && p[2].abs_diff(0xff) < 6
        }))
    };
    let mut drawn = false;
    let end = Instant::now() + Duration::from_secs(20);
    while !drawn && Instant::now() < end {
        engine.spin();
        if let Some(f) = sp.frame(&engine) {
            drawn = accent(&f);
        }
        std::thread::sleep(Duration::from_millis(30));
    }
    assert!(drawn, "the widget's button (accent colour) is drawn on the start page");
}

