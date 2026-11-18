//! Z#++ in the window: extension scripts on page loads, and the start
//! page's hero art (drawn by a bundled Z#++ program and .zsg shader).

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

use stonedgem_extensions::registry::Entry;
use stonedgem_zsharp::{Interp, Program, Value};

/// What an extension script asked the browser to do.
#[derive(Clone, Debug, PartialEq)]
pub enum Action {
    RunJs(String),
    Css(String),
    Notify(String),
}

#[derive(Default)]
struct PageCtx {
    url: String,
    title: String,
    actions: Vec<Action>,
    store: BTreeMap<String, Value>,
}

/// The loaded scripts of enabled extensions.
#[derive(Default)]
pub struct Scripts {
    progs: Vec<(String, Program, Rc<RefCell<PageCtx>>)>,
    /// Scripts that failed to load (shown in Settings).
    pub errors: Vec<String>,
}

fn api(it: &mut Interp, ctx: Rc<RefCell<PageCtx>>) {
    let c = ctx.clone();
    it.register("page_url", move |_, _| Ok(Value::str(&c.borrow().url)));
    let c = ctx.clone();
    it.register("page_title", move |_, _| Ok(Value::str(&c.borrow().title)));
    let c = ctx.clone();
    it.register("inject_css", move |_, a| {
        c.borrow_mut().actions.push(Action::Css(a.first().map(|v| v.to_string()).unwrap_or_default()));
        Ok(Value::Null)
    });
    let c = ctx.clone();
    it.register("run_js", move |_, a| {
        c.borrow_mut().actions.push(Action::RunJs(a.first().map(|v| v.to_string()).unwrap_or_default()));
        Ok(Value::Null)
    });
    let c = ctx.clone();
    it.register("notify", move |_, a| {
        let t: String = a.first().map(|v| v.to_string()).unwrap_or_default().chars().take(200).collect();
        c.borrow_mut().actions.push(Action::Notify(t));
        Ok(Value::Null)
    });
    let c = ctx.clone();
    it.register("store_get", move |_, a| Ok(c.borrow().store.get(&a.first().map(|v| v.to_string()).unwrap_or_default()).cloned().unwrap_or(Value::Null)));
    let c = ctx;
    it.register("store_set", move |_, a| {
        let k = a.first().map(|v| v.to_string()).unwrap_or_default();
        c.borrow_mut().store.insert(k, a.get(1).cloned().unwrap_or(Value::Null));
        Ok(Value::Null)
    });
}

impl Scripts {
    /// Load the scripts of the enabled extensions in `list` that have one.
    pub fn load(list: &[Entry]) -> Scripts {
        let mut s = Scripts::default();
        for e in list.iter().filter(|e| e.enabled && e.has_script) {
            let ctx = Rc::new(RefCell::new(PageCtx::default()));
            let c2 = ctx.clone();
            match stonedgem_extensions::zscript::load(std::path::Path::new(&e.path), move |it| api(it, c2)) {
                Ok(p) => s.progs.push((e.name.clone(), p, ctx)),
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
        for (name, prog, ctx) in &mut self.progs {
            {
                let mut c = ctx.borrow_mut();
                c.url = url.to_string();
                c.title = title.to_string();
                c.actions.clear();
            }
            if let Err(e) = prog.interp.emit("page_load", Some(url), vec![Value::str(url), Value::str(title)]) {
                out.push((name.clone(), Action::Notify(format!("{name}: {e}"))));
            }
            for a in ctx.borrow_mut().actions.drain(..) {
                out.push((name.clone(), a));
            }
        }
        out
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
