//! Extensions at run time, between the pages and the browser:
//!
//! * power extensions' `network.ts`, each in a hidden background page
//!   ([`ExtHub::start`]), with `oynix.network` answered here: `block`
//!   (runtime blocker rules), `setRequestHeader` (recorded; see
//!   COMPATIBILITY.md), `onRequest` (the engine's request feed) and
//!   `fetch` (no cross-origin limits; on a worker thread);
//! * `chrome.*` calls from Chrome/Firefox content scripts
//!   (`runtime.sendMessage` → the extension's Z#++ `on message`,
//!   `storage.local`, `tabs.create`).
//!
//! The window calls [`ExtHub::pump`] a few times a second.

use std::hash::{BuildHasher, Hasher};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::time::Duration;

use serde_json::{json, Value as Json};
use stonedgem_blocker::netrules;
use stonedgem_engine::{Engine, Page};
use stonedgem_extensions::registry::{self, Entry};

use crate::bridge;
use crate::zs_hooks::{css_js, Action, Scripts};

/// What the window should do after a pump.
#[derive(Debug, PartialEq)]
pub enum Effect {
    /// `chrome.tabs.create({url})`.
    OpenTab(String),
    Notify(String),
    /// Run JavaScript in the tab at this index (a Z#++ handler's request).
    RunJs(usize, String),
    /// Runtime blocker rules changed: rebuild the blocker.
    BlockerChanged,
    /// Set a property of an extension's panel: (extension, property, value).
    PanelSet(String, String, Json),
    /// Use an installed hypertheme (its file name).
    ApplyHypertheme(String),
}

pub struct Background {
    pub ext: String,
    token: String,
    pub page: Page,
}

type Fetched = (String, String, Result<Json, String>);

pub struct ExtHub {
    pub background: Vec<Background>,
    /// Whether any content script uses the `chrome.*` bridge (otherwise
    /// tabs aren't polled).
    pub content_api: bool,
    /// Extensions whose background page couldn't start, with the reason.
    pub errors: Vec<String>,
    tx: Sender<Fetched>,
    rx: Receiver<Fetched>,
}

impl Default for ExtHub {
    fn default() -> Self {
        let (tx, rx) = channel();
        ExtHub { background: vec![], content_api: false, errors: vec![], tx, rx }
    }
}

fn token() -> String {
    let mut h = std::collections::hash_map::RandomState::new().build_hasher();
    h.write_u128(std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0));
    let a = h.finish();
    let mut h = std::collections::hash_map::RandomState::new().build_hasher();
    h.write_u64(a ^ std::process::id() as u64);
    format!("{a:016x}{:016x}", h.finish())
}

/// The window's `chrome`/`browser` stand-in for one extension.
pub fn content_api(e: &Entry, manifest: &Json) -> String {
    bridge::chrome_shim(&e.id, manifest)
}

/// Scripts for every page: OyNIx-format content scripts and Chrome/Firefox
/// content scripts with the `chrome.*` bridge.
pub fn page_scripts(list: &[Entry]) -> (Vec<String>, bool) {
    let mut s = registry::page_scripts(list);
    let web = registry::webext_page_scripts(list, content_api);
    let any = !web.is_empty();
    s.extend(web);
    (s, any)
}

impl ExtHub {
    /// Start the background pages of enabled power extensions with `network.ts`.
    pub fn start(engine: &Engine, list: &[Entry]) -> ExtHub {
        let mut hub = ExtHub::default();
        for e in list.iter().filter(|e| e.enabled && e.power && e.has_network) {
            let token = token();
            let started = registry::network_html(e, &token).and_then(|html| {
                let dir = stonedgem_profile::cache_dir().join("ext-pages");
                std::fs::create_dir_all(&dir).map_err(|err| err.to_string())?;
                let file = dir.join(format!("{}-network.html", e.id.replace(['/', '\\', '.'], "_")));
                stonedgem_profile::write_atomic(&file, html.as_bytes()).map_err(|err| err.to_string())?;
                let url = format!("file://{}", file.display());
                let page = engine.open(&url, &[], &[])?;
                page.hide();
                Ok(page)
            });
            match started {
                Ok(page) => hub.background.push(Background { ext: e.id.clone(), token, page }),
                Err(err) => hub.errors.push(format!("{}: network.ts: {err}", e.name)),
            }
        }
        hub
    }

    /// Stop every background page and forget what its extension set.
    /// Returns true if blocker rules were dropped (rebuild the blocker).
    pub fn stop(&mut self) -> bool {
        let mut changed = false;
        for b in self.background.drain(..) {
            changed |= netrules::forget(&b.ext);
        }
        changed
    }

    /// Collect and answer what pages asked since the last pump. `tabs` are
    /// the window's web pages by tab index.
    pub fn pump(&mut self, engine: &Engine, tabs: &[(usize, &Page)], zs: &mut Scripts, list: &[Entry]) -> Vec<Effect> {
        let mut fx = vec![];
        let t = Duration::from_millis(400);
        // network.ts pages.
        for bi in 0..self.background.len() {
            let msgs = drain(engine, &self.background[bi].page, t);
            for (id, kind, body) in msgs {
                let b = &self.background[bi];
                if kind != "oynix:net" || id != b.ext || body["token"].as_str() != Some(b.token.as_str()) {
                    continue;
                }
                let req = body["req"].as_str().unwrap_or("").to_string();
                let reply = match body["op"].as_str().unwrap_or("") {
                    "block" => {
                        let rules = match &body["rules"] {
                            Json::Array(a) => a.iter().filter_map(|x| x.as_str()).collect::<Vec<_>>().join("\n"),
                            Json::String(s) => s.clone(),
                            _ => String::new(),
                        };
                        if netrules::set_block(&b.ext, &rules) {
                            fx.push(Effect::BlockerChanged);
                        }
                        Some(Ok(Json::Null))
                    }
                    "header" => Some(
                        netrules::add_header(&b.ext, body["pattern"].as_str().unwrap_or("*"), body["name"].as_str().unwrap_or(""), body["value"].as_str().unwrap_or(""))
                            .map(|_| Json::Null),
                    ),
                    "clearHeaders" => {
                        netrules::clear_headers(&b.ext);
                        Some(Ok(Json::Null))
                    }
                    "observe" => {
                        netrules::set_observing(&b.ext, body["on"].as_bool().unwrap_or(true));
                        Some(Ok(Json::Null))
                    }
                    "fetch" => {
                        let tx = self.tx.clone();
                        let (ext, rq) = (b.ext.clone(), req.clone());
                        std::thread::spawn(move || {
                            let _ = tx.send((ext, rq, fetch(&body)));
                        });
                        None
                    }
                    other => Some(Err(format!("unknown call “{other}”"))),
                };
                if let Some(r) = reply {
                    let _ = self.background[bi].page.eval(engine, &bridge::net_reply(&req, r), t);
                }
            }
        }
        // Finished fetches.
        while let Ok((ext, req, r)) = self.rx.try_recv() {
            if let Some(b) = self.background.iter().find(|b| b.ext == ext) {
                let _ = b.page.eval(engine, &bridge::net_reply(&req, r), t);
            }
        }
        // The request feed.
        let seen = netrules::drain();
        if seen.as_array().is_some_and(|a| !a.is_empty()) {
            // Each page's own requests aren't news to it, but the others' are;
            // OyNIx sends everything to every observer, and so do we.
            let js = bridge::net_requests(&seen);
            for b in &self.background {
                let _ = b.page.eval(engine, &js, t);
            }
        }
        // chrome.* from content scripts.
        if self.content_api {
            for &(ti, page) in tabs {
                let url = page.url().unwrap_or_default();
                for (id, kind, body) in drain(engine, page, t) {
                    if kind != "chrome" || body["secret"].as_str() != Some(bridge::ext_secret(&id).as_str()) || !list.iter().any(|e| e.id == id && e.enabled) {
                        continue;
                    }
                    let req = body["req"].as_str().unwrap_or("").to_string();
                    let op = body["op"].as_str().unwrap_or("");
                    let r = match op {
                        "sendMessage" => match zs.message(&id, body["message"].clone(), &url) {
                            Some((r, actions)) => {
                                for a in actions {
                                    fx.push(match a {
                                        Action::RunJs(js) => Effect::RunJs(ti, js),
                                        Action::Css(css) => Effect::RunJs(ti, css_js(&css)),
                                        Action::Notify(n) => Effect::Notify(n),
                                        Action::PanelSet(k, v) => Effect::PanelSet(id.clone(), k, v),
                                        Action::ApplyHypertheme(f) => Effect::ApplyHypertheme(f),
                                    });
                                }
                                r
                            }
                            None => Err("this extension has nothing listening for messages (no script.zsp)".into()),
                        },
                        "tabs.create" => {
                            let u = body["url"].as_str().unwrap_or("").to_string();
                            if u.starts_with("https://") || u.starts_with("http://") {
                                fx.push(Effect::OpenTab(u.clone()));
                                Ok(json!({ "url": u }))
                            } else {
                                Err("tabs.create: only http(s) addresses".into())
                            }
                        }
                        o if o.starts_with("storage.") => bridge::storage(&id, o, &body),
                        o => Err(format!("chrome.{o} isn't supported")),
                    };
                    let _ = page.eval(engine, &bridge::chrome_reply(&id, &req, r), t);
                }
            }
        }
        fx
    }
}

fn drain(engine: &Engine, page: &Page, t: Duration) -> Vec<(String, String, Json)> {
    match page.eval(engine, bridge::DRAIN_JS, t) {
        Ok(stonedgem_engine::Value::String(s)) => bridge::parse_drain(&s),
        _ => vec![],
    }
}

/// `oynix.network.fetch`: no cross-origin limits, http(s) only.
pub fn fetch(req: &Json) -> Result<Json, String> {
    let url = req["url"].as_str().unwrap_or("");
    if !(url.starts_with("https://") || url.starts_with("http://")) {
        return Err("only http(s) addresses can be fetched".into());
    }
    let method = req["method"].as_str().unwrap_or("GET").to_ascii_uppercase();
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(30)))
        .http_status_as_error(false)
        .user_agent(concat!("Mozilla/5.0 (X11; Linux x86_64) StonedGem/", env!("CARGO_PKG_VERSION")))
        .build()
        .into();
    let headers: Vec<(String, String)> =
        req["headers"].as_object().map(|h| h.iter().map(|(k, v)| (k.clone(), v.as_str().unwrap_or("").to_string())).collect()).unwrap_or_default();
    let body = req["body"].as_str().unwrap_or("").to_string();
    macro_rules! with_headers {
        ($r:expr) => {{
            let mut r = $r;
            for (k, v) in &headers {
                r = r.header(k.as_str(), v.as_str());
            }
            r
        }};
    }
    let resp = match method.as_str() {
        "GET" => with_headers!(agent.get(url)).call(),
        "HEAD" => with_headers!(agent.head(url)).call(),
        "DELETE" => with_headers!(agent.delete(url)).call(),
        "POST" => with_headers!(agent.post(url)).send(body.as_bytes()),
        "PUT" => with_headers!(agent.put(url)).send(body.as_bytes()),
        "PATCH" => with_headers!(agent.patch(url)).send(body.as_bytes()),
        m => return Err(format!("unsupported method {m}")),
    };
    let mut resp = resp.map_err(|e| e.to_string())?;
    let status = resp.status().as_u16();
    let headers: serde_json::Map<String, Json> =
        resp.headers().iter().map(|(k, v)| (k.as_str().to_string(), Json::String(v.to_str().unwrap_or("").to_string()))).collect();
    let text = resp.body_mut().with_config().limit(8 << 20).read_to_string().map_err(|e| e.to_string())?;
    Ok(json!({ "status": status, "headers": headers, "body": text }))
}
