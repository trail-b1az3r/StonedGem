//! The phase-4 spike, end to end: Servo loads a page from a local server,
//! the blocker stops an ad request, an extension's content script runs and
//! an element-hiding rule hides an element. No internet needed: blocked
//! requests never leave the engine.

use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use std::time::Duration;

use stonedgem_engine::{Engine, Load, Value};

/// A tiny HTTP server: `/` is the page, `/app.js` an allowed script.
fn serve() -> String {
    let l = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = l.local_addr().unwrap();
    std::thread::spawn(move || {
        for s in l.incoming().flatten() {
            let mut r = BufReader::new(s.try_clone().unwrap());
            let mut line = String::new();
            let _ = r.read_line(&mut line);
            loop {
                let mut h = String::new();
                if r.read_line(&mut h).is_err() || h == "\r\n" || h.is_empty() {
                    break;
                }
            }
            let path = line.split(' ').nth(1).unwrap_or("/");
            let (ty, body) = match path {
                "/app.js" => ("text/javascript", "window.appLoaded = true;".to_string()),
                _ => (
                    "text/html",
                    "<!DOCTYPE html><title>Spike</title>\
                     <div class=\"ad-banner\" id=\"ad\">ad</div>\
                     <script src=\"/app.js\"></script>\
                     <script src=\"https://doubleclick.net/ads.js\"></script>\
                     <img src=\"https://www.google-analytics.com/collect?v=1\">"
                        .to_string(),
                ),
            };
            let mut s = s;
            let _ = write!(s, "HTTP/1.1 200 OK\r\nContent-Type: {ty}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len());
        }
    });
    format!("http://{addr}/")
}

#[test]
fn servo_page_with_blocker_and_extension() {
    // Built-in rules plus one element-hiding rule, synchronously.
    let dir = std::env::temp_dir().join(format!("stonedgem-engine-spike-{}", std::process::id()));
    std::env::set_var("XDG_CONFIG_HOME", &dir);
    std::env::set_var("XDG_DATA_HOME", &dir);
    let filters = stonedgem_profile_filters(&dir);
    std::fs::write(filters.join("spike.txt"), "127.0.0.1###ad\n").unwrap();
    stonedgem_blocker::blocker::init(vec![]);
    let t = std::time::Instant::now();
    while stonedgem_blocker::blocker::loading() && t.elapsed() < Duration::from_secs(10) {
        std::thread::sleep(Duration::from_millis(10));
    }

    let url = serve();
    let engine = Engine::headless(800, 600).expect("software rendering (needs Mesa's EGL)");
    let page = engine
        .open(&url, &["window.extensionRan = 'yes';".to_string()], &["body { margin: 7px; }".to_string()])
        .unwrap();
    assert!(engine.wait(Duration::from_secs(60), || page.load() == Load::Complete), "page never finished loading");

    assert_eq!(page.title().as_deref(), Some("Spike"));
    let t = Duration::from_secs(10);
    assert_eq!(page.eval(&engine, "window.appLoaded === true", t), Ok(Value::Boolean(true)), "allowed script must run");
    assert_eq!(page.eval(&engine, "window.extensionRan", t), Ok(Value::String("yes".into())), "content script must run");
    assert_eq!(page.eval(&engine, "getComputedStyle(document.body).marginTop", t), Ok(Value::String("7px".into())), "content style must apply");
    assert_eq!(
        page.eval(&engine, "getComputedStyle(document.getElementById('ad')).display", t),
        Ok(Value::String("none".into())),
        "element-hiding rule must apply"
    );
    let blocked = page.blocked();
    assert!(blocked.iter().any(|u| u.contains("doubleclick.net")), "{blocked:?}");
    assert!(blocked.iter().any(|u| u.contains("google-analytics.com")), "{blocked:?}");
    assert!(!blocked.iter().any(|u| u.contains("app.js")), "{blocked:?}");
}

/// `~/.config/<app>/<profile>/filters` under `dir`.
fn stonedgem_profile_filters(dir: &std::path::Path) -> std::path::PathBuf {
    let p = dir.join("stonedgem").join("default").join("filters");
    std::fs::create_dir_all(&p).unwrap();
    p
}
