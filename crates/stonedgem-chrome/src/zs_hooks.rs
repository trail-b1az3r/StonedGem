//! Extension scripts in the window — Z#++ (`script.zsp`) or Zsharpend
//! (`script.zxx`) — on page loads, messages and panel callbacks; and the
//! start page's hero art (drawn by a bundled Z#++ program and .zsg shader).
//!
//! Both engines see the same API; values cross into the browser as JSON.
//! Power extensions' Zsharpend scripts also get the Theme Studio functions
//! (`to_json`, `zsg_preview`, `hypertheme_save`, `hypertheme_apply`…).

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

use serde_json::Value as Json;
use stonedgem_extensions::registry::Entry;

/// What an extension script asked the browser to do.
#[derive(Clone, Debug, PartialEq)]
pub enum Action {
    RunJs(String),
    Css(String),
    Notify(String),
    /// Set a property of the extension's Slint panel.
    PanelSet(String, Json),
    /// Use an installed hypertheme (file name in the profile's hyperthemes folder).
    ApplyHypertheme(String),
}

#[derive(Default)]
struct PageCtx {
    url: String,
    title: String,
    actions: Vec<Action>,
    store: BTreeMap<String, Json>,
}

enum Engine {
    Zsp(stonedgem_zsharp::Program),
    Zxx(zsharpend::Program),
}

impl Engine {
    /// Emit `event`; each handler's return value, as JSON.
    fn emit(&mut self, event: &str, subject: Option<&str>, args: Vec<Json>) -> Result<Vec<Json>, String> {
        match self {
            Engine::Zsp(p) => {
                let a = args.iter().map(crate::bridge::from_json).collect();
                p.interp.emit_values(event, subject, a).map(|vs| vs.iter().map(crate::bridge::to_json).collect())
            }
            Engine::Zxx(p) => {
                let a = args.iter().map(stonedgem_hypertheme::json_to_zxx).collect();
                p.interp.emit_values(event, subject, a).map(|vs| vs.iter().map(stonedgem_hypertheme::zxx_to_json).collect())
            }
        }
    }
}

/// The loaded scripts of enabled extensions.
#[derive(Default)]
pub struct Scripts {
    /// (extension id, name, engine, context)
    progs: Vec<(String, String, Engine, Rc<RefCell<PageCtx>>)>,
    /// Scripts that failed to load (shown in Settings).
    pub errors: Vec<String>,
}

/// The extension API, for either engine (their `Interp` and `Value` have
/// the same shape). `$to`/`$from` convert values to and from JSON.
macro_rules! register_api {
    ($it:expr, $ctx:expr, $V:ty, $to:path, $from:path) => {{
        let it = $it;
        let ctx: Rc<RefCell<PageCtx>> = $ctx;
        let text = |a: &[$V], i: usize| a.get(i).map(|v| v.to_string()).unwrap_or_default();
        let c = ctx.clone();
        it.register("page_url", move |_, _| Ok(<$V>::str(&c.borrow().url)));
        let c = ctx.clone();
        it.register("page_title", move |_, _| Ok(<$V>::str(&c.borrow().title)));
        let c = ctx.clone();
        it.register("inject_css", move |_, a| {
            c.borrow_mut().actions.push(Action::Css(text(&a, 0)));
            Ok(<$V>::Null)
        });
        let c = ctx.clone();
        it.register("run_js", move |_, a| {
            c.borrow_mut().actions.push(Action::RunJs(text(&a, 0)));
            Ok(<$V>::Null)
        });
        let c = ctx.clone();
        it.register("notify", move |_, a| {
            let t: String = text(&a, 0).chars().take(200).collect();
            c.borrow_mut().actions.push(Action::Notify(t));
            Ok(<$V>::Null)
        });
        let c = ctx.clone();
        it.register("panel_set", move |_, a| {
            c.borrow_mut().actions.push(Action::PanelSet(text(&a, 0), a.get(1).map($to).unwrap_or(Json::Null)));
            Ok(<$V>::Null)
        });
        let c = ctx.clone();
        it.register("store_get", move |_, a| Ok(c.borrow().store.get(&text(&a, 0)).map($from).unwrap_or(<$V>::Null)));
        let c = ctx.clone();
        it.register("store_set", move |_, a| {
            let v = a.get(1).map($to).unwrap_or(Json::Null);
            c.borrow_mut().store.insert(text(&a, 0), v);
            Ok(<$V>::Null)
        });
    }};
}

fn zsp_api(it: &mut stonedgem_zsharp::Interp, ctx: Rc<RefCell<PageCtx>>) {
    register_api!(it, ctx, stonedgem_zsharp::Value, crate::bridge::to_json, crate::bridge::from_json);
}

fn zxx_api(it: &mut zsharpend::Interp, ctx: Rc<RefCell<PageCtx>>, power: bool) {
    use zsharpend::Value as V;
    register_api!(it, ctx.clone(), V, stonedgem_hypertheme::zxx_to_json, stonedgem_hypertheme::json_to_zxx);
    if !power {
        return;
    }
    // ── Power extensions: what Theme Studio needs ──
    it.register("to_json", |_, a| Ok(V::str(&serde_json::to_string(&a.first().map(stonedgem_hypertheme::zxx_to_json).unwrap_or(Json::Null)).unwrap_or_default())));
    it.register("from_json", |_, a| {
        let t = a.first().map(|v| v.to_string()).unwrap_or_default();
        serde_json::from_str::<Json>(&t).map(|j| stonedgem_hypertheme::json_to_zxx(&j)).map_err(|e| format!("from_json: {e}"))
    });
    it.register("zsg_preview", |_, a| {
        let code = a.first().map(|v| v.to_string()).unwrap_or_default();
        let w = a.get(1).and_then(|v| v.as_i64()).unwrap_or(160).clamp(8, 512);
        let h = a.get(2).and_then(|v| v.as_i64()).unwrap_or(100).clamp(8, 512);
        zsg_preview(&code, w as u32, h as u32).map(|u| V::str(&u))
    });
    it.register("hypertheme_save", |_, a| {
        let name = a.first().map(|v| v.to_string()).unwrap_or_default();
        let files = match a.get(1).map(stonedgem_hypertheme::zxx_to_json) {
            Some(Json::Object(m)) => m.into_iter().map(|(k, v)| (k, v.as_str().map(String::from).unwrap_or_else(|| v.to_string()))).collect::<BTreeMap<String, String>>(),
            _ => return Err("hypertheme_save(name, files): files is a map of file name → text".into()),
        };
        save_hypertheme(&name, files).map(|f| V::str(&f))
    });
    let c = ctx;
    it.register("hypertheme_apply", move |_, a| {
        c.borrow_mut().actions.push(Action::ApplyHypertheme(a.first().map(|v| v.to_string()).unwrap_or_default()));
        Ok(V::Null)
    });
    it.register("hypertheme_resolve", |_, a| {
        // (files, dark) → { tokens, panels, css, warnings }: what the browser
        // would make of these parts, for the Studio's live preview.
        let files = match a.first().map(stonedgem_hypertheme::zxx_to_json) {
            Some(Json::Object(m)) => m.into_iter().map(|(k, v)| (k, v.as_str().map(String::from).unwrap_or_else(|| v.to_string()))).collect::<BTreeMap<String, String>>(),
            _ => return Err("hypertheme_resolve(files, dark): files is a map of file name → text".into()),
        };
        let dark = matches!(a.get(1), Some(V::Bool(true)));
        let mut files = files;
        files.entry("hypertheme.toml".into()).or_insert_with(|| "[hypertheme]\nname = \"Preview\"\n".into());
        let h = stonedgem_hypertheme::Hypertheme::from_files(files)?;
        let r = h.resolve(dark);
        let mut css = r.css();
        if let Some(extra) = h.files.get("pages.css") {
            css.push_str(extra);
        }
        Ok(stonedgem_hypertheme::json_to_zxx(&serde_json::json!({ "tokens": r.tokens, "panels": r.panels, "css": css, "warnings": r.warnings })))
    });
    it.register("hypertheme_list", |_, _| {
        Ok(V::list(stonedgem_hypertheme::installed().into_iter().map(|(f, n)| V::list(vec![V::str(&f), V::str(&n)])).collect()))
    });
}

/// A `.zsg` with `fragment preview(uv: vec2) -> color`, drawn small, as a
/// `data:` PNG URL (Theme Studio's shader node).
pub fn zsg_preview(code: &str, w: u32, h: u32) -> Result<String, String> {
    let m = stonedgem_zsharp::zsg::Module::parse(code)?;
    let entry = if m.entry("preview").is_some() { "preview".to_string() } else { m.entry_names().into_iter().next().ok_or("no fragment to preview")? };
    let c = stonedgem_zsharp::canvas::Canvas::new(w as i64, h as i64)?;
    c.shade(&m, &entry, (0, 0, w as i32, h as i32), &Default::default())?;
    Ok(format!("data:image/png;base64,{}", base64(&c.png()?)))
}

/// Write a hypertheme into the profile (checked first). Returns its file name.
pub fn save_hypertheme(name: &str, mut files: BTreeMap<String, String>) -> Result<String, String> {
    if name.trim().is_empty() {
        return Err("a hypertheme needs a name".into());
    }
    files.entry("hypertheme.toml".into()).or_insert_with(|| format!("[hypertheme]\nname = {}\n", serde_json::to_string(name).unwrap_or_default()));
    let h = stonedgem_hypertheme::Hypertheme::from_files(files)?;
    let dir = std::env::temp_dir().join(format!("stonedgem-ht-{}", std::process::id()));
    let tmp = dir.join("theme.hypertheme");
    h.write(&tmp)?;
    let file = stonedgem_hypertheme::install(&tmp)?;
    let _ = std::fs::remove_dir_all(dir);
    Ok(file)
}

impl Scripts {
    /// Load the scripts of the enabled extensions in `list` that have one.
    pub fn load(list: &[Entry]) -> Scripts {
        let mut s = Scripts::default();
        for e in list.iter().filter(|e| e.enabled && e.has_script) {
            let ctx = Rc::new(RefCell::new(PageCtx::default()));
            let dir = std::path::Path::new(&e.path);
            let loaded = if stonedgem_extensions::zscript::is_zxx(dir) {
                let (c2, power) = (ctx.clone(), e.power);
                stonedgem_extensions::zscript::load_zxx(dir, move |it| zxx_api(it, c2, power)).map(Engine::Zxx)
            } else {
                let c2 = ctx.clone();
                stonedgem_extensions::zscript::load(dir, move |it| zsp_api(it, c2)).map(Engine::Zsp)
            };
            match loaded {
                Ok(p) => s.progs.push((e.id.clone(), e.name.clone(), p, ctx)),
                Err(err) => s.errors.push(format!("{}: {err}", e.name)),
            }
        }
        s
    }

    pub fn len(&self) -> usize {
        self.progs.len()
    }

    pub fn is_empty(&self) -> bool {
        self.progs.is_empty()
    }

    /// A page finished loading: run every script's matching `page_load`
    /// handlers and collect what they asked for. A failing script becomes
    /// a notification naming it.
    pub fn page_loaded(&mut self, url: &str, title: &str) -> Vec<(String, Action)> {
        let mut out = vec![];
        for (_, name, prog, ctx) in &mut self.progs {
            {
                let mut c = ctx.borrow_mut();
                c.url = url.to_string();
                c.title = title.to_string();
                c.actions.clear();
            }
            if let Err(e) = prog.emit("page_load", Some(url), vec![Json::String(url.into()), Json::String(title.into())]) {
                out.push((name.clone(), Action::Notify(format!("{name}: {e}"))));
            }
            for a in ctx.borrow_mut().actions.drain(..) {
                out.push((name.clone(), a));
            }
        }
        out
    }

    /// A content script (or the extension's page) called
    /// `chrome.runtime.sendMessage(msg)` from `url`: run the script's
    /// `on message (msg, sender)` handlers. The first non-null `return`
    /// value is the reply. `None`: the extension has no script.
    pub fn message(&mut self, ext: &str, msg: Json, url: &str) -> Option<(Result<Json, String>, Vec<Action>)> {
        let sender = serde_json::json!({ "url": url });
        self.event(ext, "message", vec![msg, sender], Some(url))
    }

    /// Extension `ext`'s Slint panel invoked callback `name`: run its
    /// script's `on panel (name, args)` handlers.
    pub fn panel(&mut self, ext: &str, name: &str, args: Vec<Json>) -> Option<(Result<Json, String>, Vec<Action>)> {
        self.event(ext, "panel", vec![Json::String(name.into()), Json::Array(args)], None)
    }

    fn event(&mut self, ext: &str, event: &str, args: Vec<Json>, url: Option<&str>) -> Option<(Result<Json, String>, Vec<Action>)> {
        let (_, name, prog, ctx) = self.progs.iter_mut().find(|p| p.0 == ext)?;
        {
            let mut c = ctx.borrow_mut();
            if let Some(u) = url {
                c.url = u.to_string();
            }
            c.actions.clear();
        }
        let r = prog.emit(event, None, args).map(|vs| vs.into_iter().find(|v| !v.is_null()).unwrap_or(Json::Null)).map_err(|e| format!("{name}: {e}"));
        let actions = ctx.borrow_mut().actions.drain(..).collect();
        Some((r, actions))
    }
}

/// JavaScript that adds `css` to the page as a style sheet.
pub fn css_js(css: &str) -> String {
    format!(
        "(function(){{var s=document.createElement('style');s.dataset.zsharp='1';s.textContent={};(document.head||document.documentElement).appendChild(s);}})()",
        serde_json::to_string(css).unwrap_or_else(|_| "\"\"".into())
    )
}

/// The start page's hero art: drawn once by the bundled Z#++ program, as
/// a `data:` PNG URL. Empty if it fails (the page still works).
pub fn hero_data_url() -> &'static str {
    static HERO: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    HERO.get_or_init(|| {
        // Cached in the profile, keyed by the program's text.
        let key = {
            let src = format!("{}{}", include_str!("../assets/start.zsp"), include_str!("../assets/start.zsg"));
            src.bytes().fold(0xcbf2_9ce4_8422_2325u64, |h, b| (h ^ b as u64).wrapping_mul(0x0100_0000_01b3))
        };
        let file = stonedgem_profile::cache_dir().join(format!("start-hero-{key:016x}.png"));
        let png = match std::fs::read(&file) {
            Ok(b) if b.starts_with(b"\x89PNG") => Ok(b),
            _ => hero_png().inspect(|b| {
                let _ = stonedgem_profile::write_atomic(&file, b);
            }),
        };
        png.map(|png| format!("data:image/png;base64,{}", base64(&png))).unwrap_or_default()
    })
}

pub fn hero_png() -> Result<Vec<u8>, String> {
    let read = |p: &str| match p {
        "start.zsp" => Some(include_str!("../assets/start.zsp").to_string()),
        "start.zsg" => Some(include_str!("../assets/start.zsg").to_string()),
        _ => None,
    };
    let mut p = Program::load("start.zsp", &read, |_| {})?;
    p.run_main()?;
    match p.interp.global("canvas") {
        Some(Value::Object(o)) => o.as_any().downcast_ref::<stonedgem_zsharp::canvas::Canvas>().ok_or("not a canvas".to_string())?.png(),
        _ => Err("the hero program made no canvas".into()),
    }
}

/// Standard base64 (for `data:` URLs).
pub fn base64_encode(data: &[u8]) -> String {
    base64(data)
}

fn base64(data: &[u8]) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
    for c in data.chunks(3) {
        let n = (c[0] as u32) << 16 | (*c.get(1).unwrap_or(&0) as u32) << 8 | *c.get(2).unwrap_or(&0) as u32;
        out.push(T[(n >> 18) as usize & 63] as char);
        out.push(T[(n >> 12) as usize & 63] as char);
        out.push(if c.len() > 1 { T[(n >> 6) as usize & 63] as char } else { '=' });
        out.push(if c.len() > 2 { T[n as usize & 63] as char } else { '=' });
    }
    out
}

/// The playground's starting program.
pub const PLAYGROUND_SAMPLE: &str = r#"// Z#++ playground: edit and press Run.
struct Ball { x: float, y: float, r: float }

func Main() {
    global let canvas = Canvas(480, 220)
    canvas.clear(#10141c)
    let balls = range(0, 12) |> map(|i| Ball { x: 30 + i * 38, y: 110 + sin(i) * 60, r: 8 + i % 4 * 4 })
    for i, b in balls {
        canvas.circle(b.x, b.y, b.r, hsl(200 + i * 12, 0.8, 0.6))
    }
    canvas.text(16, 16, "Hello from Z#++", 14, #ffffff)
    Printl("drew ${len(balls)} balls")
}
"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hero_renders() {
        let png = hero_png().unwrap();
        assert!(png.starts_with(b"\x89PNG"));
        assert!(hero_data_url().starts_with("data:image/png;base64,iVBOR"));
    }

    #[test]
    fn base64_matches_rfc4648() {
        assert_eq!(base64(b""), "");
        assert_eq!(base64(b"f"), "Zg==");
        assert_eq!(base64(b"fo"), "Zm8=");
        assert_eq!(base64(b"foobar"), "Zm9vYmFy");
    }

    #[test]
    fn playground_sample_runs() {
        let mut p = Program::from_source(PLAYGROUND_SAMPLE).unwrap();
        p.run_main().unwrap();
        assert_eq!(p.output(), "drew 12 balls\n");
    }
}
