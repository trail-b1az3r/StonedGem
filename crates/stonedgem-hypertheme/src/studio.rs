//! Theme Studio's host side, shared by every browser that runs it
//! (StonedGem through its extension runtime, OyNIx directly).
//!
//! * [`register_power`] adds the power-extension functions the Studio's
//!   Zsharpend script calls (`zsg_preview`, `hypertheme_resolve`,
//!   `hypertheme_save`, `hypertheme_apply`, `hypertheme_list`,
//!   `to_json`, `from_json`).
//! * [`Studio`] loads the bundled `script.zxx` on its own and answers the
//!   page's messages, for a browser without StonedGem's extension runtime.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

use serde_json::{json, Value as Json};
use zsharpend::Value as V;

use crate::{json_to_zxx, zxx_to_json, Hypertheme};

/// What the Studio asked the browser to do.
#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    /// Use this installed hypertheme (its file name).
    Apply(String),
    Notify(String),
}

fn files_arg(v: Option<&V>, what: &str) -> Result<BTreeMap<String, String>, String> {
    match v.map(zxx_to_json) {
        Some(Json::Object(m)) => Ok(m.into_iter().map(|(k, v)| (k, v.as_str().map(String::from).unwrap_or_else(|| v.to_string()))).collect()),
        _ => Err(format!("{what}: files is a map of file name → text")),
    }
}

/// Add the Studio's power functions to a Zsharpend interpreter. `apply` is
/// called when the script asks for a theme to be used.
pub fn register_power(it: &mut zsharpend::Interp, apply: Rc<dyn Fn(String)>) {
    it.register("to_json", |_, a| Ok(V::str(&serde_json::to_string(&a.first().map(zxx_to_json).unwrap_or(Json::Null)).unwrap_or_default())));
    it.register("from_json", |_, a| {
        let t = a.first().map(|v| v.to_string()).unwrap_or_default();
        serde_json::from_str::<Json>(&t).map(|j| json_to_zxx(&j)).map_err(|e| format!("from_json: {e}"))
    });
    it.register("zsg_preview", |_, a| {
        let code = a.first().map(|v| v.to_string()).unwrap_or_default();
        let w = a.get(1).and_then(|v| v.as_i64()).unwrap_or(160).clamp(8, 512);
        let h = a.get(2).and_then(|v| v.as_i64()).unwrap_or(100).clamp(8, 512);
        zsg_preview(&code, w as u32, h as u32).map(|u| V::str(&u))
    });
    it.register("hypertheme_resolve", |_, a| {
        let files = files_arg(a.first(), "hypertheme_resolve(files, dark)")?;
        preview(files, matches!(a.get(1), Some(V::Bool(true)))).map(|j| json_to_zxx(&j))
    });
    it.register("hypertheme_save", |_, a| {
        let name = a.first().map(|v| v.to_string()).unwrap_or_default();
        save(&name, files_arg(a.get(1), "hypertheme_save(name, files)")?).map(|f| V::str(&f))
    });
    it.register("hypertheme_apply", move |_, a| {
        apply(a.first().map(|v| v.to_string()).unwrap_or_default());
        Ok(V::Null)
    });
    it.register("hypertheme_list", |_, _| Ok(V::list(crate::available().into_iter().map(|(f, n)| V::list(vec![V::str(&f), V::str(&n)])).collect())));
}

/// What the browser would make of these parts: { tokens, panels, css, warnings }.
pub fn preview(mut files: BTreeMap<String, String>, dark: bool) -> Result<Json, String> {
    files.entry("hypertheme.toml".into()).or_insert_with(|| "[hypertheme]\nname = \"Preview\"\n".into());
    let h = Hypertheme::from_files(files)?;
    let r = h.resolve(dark);
    let mut css = r.css();
    if let Some(extra) = h.files.get("pages.css") {
        css.push_str(extra);
    }
    Ok(json!({ "tokens": r.tokens, "panels": r.panels, "css": css, "warnings": r.warnings }))
}

/// A `.zsg` with `fragment preview(uv: vec2) -> color` (or its first
/// fragment), drawn small, as a `data:` PNG URL.
pub fn zsg_preview(code: &str, w: u32, h: u32) -> Result<String, String> {
    let m = stonedgem_zsharp::zsg::Module::parse(code)?;
    let entry = if m.entry("preview").is_some() { "preview".to_string() } else { m.entry_names().into_iter().next().ok_or("no fragment to preview")? };
    let c = stonedgem_zsharp::canvas::Canvas::new(w as i64, h as i64)?;
    c.shade(&m, &entry, (0, 0, w as i32, h as i32), &Default::default())?;
    Ok(format!("data:image/png;base64,{}", base64(&c.png()?)))
}

/// Write a hypertheme into the profile (checked first). Returns its file name.
pub fn save(name: &str, mut files: BTreeMap<String, String>) -> Result<String, String> {
    if name.trim().is_empty() {
        return Err("a hypertheme needs a name".into());
    }
    files.entry("hypertheme.toml".into()).or_insert_with(|| format!("[hypertheme]\nname = {}\n", serde_json::to_string(name).unwrap_or_default()));
    let h = Hypertheme::from_files(files)?;
    let dir = std::env::temp_dir().join(format!("stonedgem-ht-{}-{}", std::process::id(), name.len()));
    let tmp = dir.join("theme.hypertheme");
    let _ = std::fs::create_dir_all(&dir);
    h.write(&tmp)?;
    let file = crate::install(&tmp);
    let _ = std::fs::remove_dir_all(dir);
    file
}

/// Standard base64.
pub fn base64(data: &[u8]) -> String {
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

/// Theme Studio's script, run by the browser itself: [`Studio::message`]
/// answers what the page sends with `chrome.runtime.sendMessage`.
pub struct Studio {
    prog: zsharpend::Program,
    events: Rc<RefCell<Vec<Event>>>,
}

impl Studio {
    pub fn new() -> Result<Studio, String> {
        let files = crate::studio_files();
        let read = move |p: &str| files.iter().find(|(f, _)| *f == p).map(|(_, t)| t.clone());
        let events = Rc::new(RefCell::new(vec![]));
        let ev = events.clone();
        let mut prog = zsharpend::Program::load("script.zxx", &read, move |it| {
            // The extension API the script may touch: only `notify` matters here.
            for (name, _) in stonedgem_extensions::zscript::API {
                it.register(name, |_, _| Ok(V::Null));
            }
            let n = ev.clone();
            it.register("notify", move |_, a| {
                n.borrow_mut().push(Event::Notify(a.first().map(|v| v.to_string()).unwrap_or_default()));
                Ok(V::Null)
            });
            let ap = ev.clone();
            register_power(it, Rc::new(move |f| ap.borrow_mut().push(Event::Apply(f))));
        })?;
        prog.interp.limits.time = std::time::Duration::from_secs(10);
        Ok(Studio { prog, events })
    }

    /// Run the script's `on message (msg, sender)`: its reply, and what it
    /// asked the browser to do.
    pub fn message(&mut self, msg: Json) -> (Result<Json, String>, Vec<Event>) {
        self.events.borrow_mut().clear();
        let args = vec![json_to_zxx(&msg), json_to_zxx(&json!({ "url": "oynix://theme-studio" }))];
        let r = self.prog.interp.emit_values("message", None, args).map(|vs| vs.iter().map(zxx_to_json).find(|v| !v.is_null()).unwrap_or(Json::Null));
        (r, self.events.borrow_mut().drain(..).collect())
    }
}
