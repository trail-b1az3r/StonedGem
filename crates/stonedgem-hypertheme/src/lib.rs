//! **.hypertheme** — a whole-UI theme for StonedGem (and OyNIx): one set
//! of design tokens that every layer reads, plus parts for the layers that
//! need more than tokens.
//!
//! A `.hypertheme` is a zip archive (or a folder) holding
//! `hypertheme.toml` and any of:
//!
//! | File | What it gives |
//! |---|---|
//! | `tokens.json` | tokens: `{ "background": "#…", …, "panels": { "card": {…} }, "dark": { … } }` |
//! | `style.rs` | tokens as **basic Rust** constants (`pub const ACCENT: Color = rgb(255, 222, 89);`, a `mod dark { … }` for dark mode) |
//! | `theme.zsp` | a **Z#++** program: `func tokens(dark)` returns a map of tokens |
//! | `theme.zxx` | a **Zsharpend** program: `fn tokens(dark)` returns a map of tokens |
//! | `iced.toml` | the Iced window's panels: `[toolbar] fill = "#…" border_width = 3 …` |
//! | `pages.css` | CSS for the start page and extension pages (after the generated `--ht-*` variables) |
//! | `start.tsx` | a **TSX** start page (`export default function Start() { … }`) |
//! | `chrome.qml` | **QML** for OyNIx (a `Theme` override) |
//! | `background.zsg` | a fragment shader (`fragment background(uv: vec2) -> color`) drawn behind the start page |
//! | `graph.json` | the Theme Studio graph it was made from, to edit it again |
//!
//! Tokens are merged in that order — `tokens.json`, `style.rs`,
//! `theme.zsp`, `theme.zxx`, `iced.toml` — later ones winning, then the
//! `dark` overrides when the browser is dark. Theme programs run in the
//! Z#++/Zsharpend sandboxes.

use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::path::Path;

use serde::Deserialize;
use serde_json::{json, Map, Value as Json};

pub mod basic_rust;

/// The files a hypertheme may hold.
pub const FILES: &[&str] = &[
    "hypertheme.toml", "tokens.json", "style.rs", "theme.zsp", "theme.zxx", "iced.toml", "pages.css", "start.tsx", "chrome.qml", "background.zsg", "graph.json",
];
const MAX_FILE: usize = 1 << 20;
const MAX_TOTAL: usize = 4 << 20;

/// Global tokens every theme can set (colours are `#rrggbb`).
pub const COLOR_TOKENS: &[&str] = &["background", "surface", "surface_alt", "text", "muted", "accent", "on_accent", "border", "danger", "shadow"];
pub const NUMBER_TOKENS: &[&str] = &["radius", "border_width", "shadow_x", "shadow_y", "shadow_blur", "font_weight"];
/// The panels the window draws, each with its own style.
pub const PANELS: &[&str] = &["toolbar", "tab", "card", "button", "field", "popup"];
/// What a panel's style may set.
pub const PANEL_KEYS: &[&str] = &["fill", "text", "border", "radius", "border_width", "shadow_x", "shadow_y"];

#[derive(Debug, Deserialize, Clone, Default)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    pub name: String,
    #[serde(default = "v010")]
    pub version: String,
    #[serde(default)]
    pub author: String,
    #[serde(default)]
    pub description: String,
    /// Which modes it was made for: "light", "dark" or "both".
    #[serde(default = "both")]
    pub modes: String,
}
fn v010() -> String {
    "0.1.0".into()
}
fn both() -> String {
    "both".into()
}

#[derive(Debug, Deserialize)]
struct ManifestFile {
    hypertheme: Manifest,
}

/// A loaded hypertheme: its manifest and files.
#[derive(Clone, Debug)]
pub struct Hypertheme {
    pub manifest: Manifest,
    pub files: BTreeMap<String, String>,
}

/// Tokens for one mode, ready to use.
#[derive(Clone, Debug, Default)]
pub struct Resolved {
    pub dark: bool,
    pub tokens: Map<String, Json>,
    pub panels: BTreeMap<String, Map<String, Json>>,
    /// Problems worth showing (contrast too low, a theme program failed…).
    pub warnings: Vec<String>,
}

impl Hypertheme {
    /// From a `.hypertheme` file (zip) or a folder.
    pub fn open(path: &Path) -> Result<Hypertheme, String> {
        let files = if path.is_dir() { read_dir(path)? } else { read_zip(&std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?)? };
        Hypertheme::from_files(files)
    }

    pub fn from_files(files: BTreeMap<String, String>) -> Result<Hypertheme, String> {
        let mut total = 0;
        for (name, text) in &files {
            if !FILES.contains(&name.as_str()) {
                return Err(format!("a hypertheme can't hold `{name}` (it holds {})", FILES.join(", ")));
            }
            if text.len() > MAX_FILE {
                return Err(format!("{name} is larger than 1 MB"));
            }
            total += text.len();
        }
        if total > MAX_TOTAL {
            return Err("a hypertheme is at most 4 MB".into());
        }
        let m = files.get("hypertheme.toml").ok_or("a hypertheme needs hypertheme.toml")?;
        let manifest = toml::from_str::<ManifestFile>(m).map_err(|e| format!("hypertheme.toml: {e}"))?.hypertheme;
        if manifest.name.trim().is_empty() {
            return Err("hypertheme.toml: `name` can't be empty".into());
        }
        let h = Hypertheme { manifest, files };
        h.check()?;
        Ok(h)
    }

    /// Parse every part, so a broken theme is refused when it's installed.
    pub fn check(&self) -> Result<(), String> {
        if let Some(t) = self.files.get("tokens.json") {
            serde_json::from_str::<Json>(t).map_err(|e| format!("tokens.json: {e}"))?;
        }
        if let Some(t) = self.files.get("style.rs") {
            basic_rust::parse(t).map_err(|e| format!("style.rs: {e}"))?;
        }
        if let Some(t) = self.files.get("iced.toml") {
            toml::from_str::<toml::Table>(t).map_err(|e| format!("iced.toml: {e}"))?;
        }
        if let Some(t) = self.files.get("theme.zsp") {
            stonedgem_zsharp::parser::parse(t, stonedgem_zsharp::ast::Kind::Program).map_err(|e| format!("theme.zsp: {e}"))?;
        }
        if let Some(t) = self.files.get("theme.zxx") {
            zsharpend::parser::parse(t, zsharpend::ast::Kind::Zxx).map_err(|e| format!("theme.zxx: {e}"))?;
        }
        if let Some(t) = self.files.get("background.zsg") {
            let m = stonedgem_zsharp::zsg::Module::parse(t).map_err(|e| format!("background.zsg: {e}"))?;
            if m.entry("background").is_none() {
                return Err("background.zsg: needs `fragment background(uv: vec2) -> color`".into());
            }
        }
        if let Some(t) = self.files.get("start.tsx") {
            stonedgem_extensions::tscompile::transpile_tsx(t, "start.tsx")?;
        }
        if let Some(t) = self.files.get("graph.json") {
            serde_json::from_str::<Json>(t).map_err(|e| format!("graph.json: {e}"))?;
        }
        Ok(())
    }

    /// The tokens for light or dark mode.
    pub fn resolve(&self, dark: bool) -> Resolved {
        let mut r = Resolved { dark, ..Default::default() };
        let mut layers: Vec<(String, Json)> = vec![];
        if let Some(t) = self.files.get("tokens.json") {
            if let Ok(v) = serde_json::from_str::<Json>(t) {
                layers.push(("tokens.json".into(), v));
            }
        }
        if let Some(t) = self.files.get("style.rs") {
            match basic_rust::parse(t) {
                Ok(v) => layers.push(("style.rs".into(), v)),
                Err(e) => r.warnings.push(format!("style.rs: {e}")),
            }
        }
        if let Some(t) = self.files.get("theme.zsp") {
            match run_zsp(t, dark) {
                Ok(v) => layers.push(("theme.zsp".into(), v)),
                Err(e) => r.warnings.push(format!("theme.zsp: {e}")),
            }
        }
        if let Some(t) = self.files.get("theme.zxx") {
            match run_zxx(t, dark) {
                Ok(v) => layers.push(("theme.zxx".into(), v)),
                Err(e) => r.warnings.push(format!("theme.zxx: {e}")),
            }
        }
        if let Some(t) = self.files.get("iced.toml") {
            if let Ok(tb) = toml::from_str::<toml::Table>(t) {
                let v = serde_json::to_value(&tb).unwrap_or(Json::Null);
                layers.push(("iced.toml".into(), json!({ "panels": v })));
            }
        }
        for (_, layer) in &layers {
            merge(&mut r, layer);
        }
        if dark {
            for (_, layer) in &layers {
                if let Some(d) = layer.get("dark") {
                    merge(&mut r, d);
                }
            }
        }
        r.fill_defaults();
        r.warnings.extend(r.contrast_warnings());
        r
    }

    /// Write as a `.hypertheme` (zip).
    pub fn write(&self, path: &Path) -> Result<(), String> {
        write_zip(path, &self.files)
    }
}

fn merge(r: &mut Resolved, layer: &Json) {
    let Some(obj) = layer.as_object() else { return };
    for (k, v) in obj {
        match k.as_str() {
            "dark" | "light" => {}
            "panels" => {
                if let Some(ps) = v.as_object() {
                    for (p, style) in ps {
                        if let Some(s) = style.as_object() {
                            let slot = r.panels.entry(p.clone()).or_default();
                            for (sk, sv) in s {
                                slot.insert(sk.clone(), sv.clone());
                            }
                        }
                    }
                }
            }
            _ => {
                r.tokens.insert(k.clone(), v.clone());
            }
        }
    }
}

fn hex_rgb(s: &str) -> Option<(u8, u8, u8)> {
    let h = s.strip_prefix('#')?;
    let v = u32::from_str_radix(h.get(..6)?, 16).ok()?;
    Some(((v >> 16) as u8, (v >> 8) as u8, v as u8))
}

/// WCAG contrast ratio of two `#rrggbb` colours.
pub fn contrast(a: &str, b: &str) -> Option<f64> {
    let lum = |c: (u8, u8, u8)| {
        let f = |x: u8| {
            let s = x as f64 / 255.0;
            if s <= 0.03928 { s / 12.92 } else { ((s + 0.055) / 1.055).powf(2.4) }
        };
        0.2126 * f(c.0) + 0.7152 * f(c.1) + 0.0722 * f(c.2)
    };
    let (x, y) = (lum(hex_rgb(a)?), lum(hex_rgb(b)?));
    Some((x.max(y) + 0.05) / (x.min(y) + 0.05))
}

impl Resolved {
    pub fn color(&self, key: &str) -> Option<String> {
        self.tokens.get(key).and_then(Json::as_str).filter(|s| hex_rgb(s).is_some()).map(str::to_string)
    }
    pub fn number(&self, key: &str) -> Option<f64> {
        self.tokens.get(key).and_then(Json::as_f64)
    }
    pub fn flag(&self, key: &str) -> Option<bool> {
        self.tokens.get(key).and_then(Json::as_bool)
    }
    /// A panel's style value, falling back to the global token.
    pub fn panel(&self, panel: &str, key: &str) -> Option<Json> {
        self.panels.get(panel).and_then(|p| p.get(key)).cloned()
    }

    /// Missing colours are derived from the ones given, so a theme only
    /// has to set a few.
    fn fill_defaults(&mut self) {
        let dark = self.dark;
        let get = |t: &Map<String, Json>, k: &str| t.get(k).and_then(Json::as_str).map(String::from);
        let bg = get(&self.tokens, "background").unwrap_or_else(|| if dark { "#16181c".into() } else { "#fafafa".into() });
        let text = get(&self.tokens, "text").unwrap_or_else(|| if dark { "#e8e8ea".into() } else { "#18181b".into() });
        let defaults = [
            ("background", bg.clone()),
            ("text", text.clone()),
            ("surface", get(&self.tokens, "surface").unwrap_or_else(|| bg.clone())),
            ("surface_alt", get(&self.tokens, "surface_alt").or_else(|| get(&self.tokens, "surface")).unwrap_or_else(|| bg.clone())),
            ("muted", get(&self.tokens, "muted").unwrap_or_else(|| text.clone())),
            ("accent", get(&self.tokens, "accent").unwrap_or_else(|| "#2f7cf6".into())),
            ("on_accent", get(&self.tokens, "on_accent").unwrap_or_else(|| "#ffffff".into())),
            ("border", get(&self.tokens, "border").unwrap_or_else(|| text.clone())),
            ("danger", get(&self.tokens, "danger").unwrap_or_else(|| "#d93025".into())),
            ("shadow", get(&self.tokens, "shadow").unwrap_or_else(|| "#000000".into())),
        ];
        for (k, v) in defaults {
            self.tokens.entry(k).or_insert(Json::String(v));
        }
        for (k, v) in [("radius", 12.0), ("border_width", 1.0), ("shadow_x", 0.0), ("shadow_y", 0.0), ("shadow_blur", 0.0), ("font_weight", 400.0)] {
            self.tokens.entry(k).or_insert(json!(v));
        }
        self.tokens.entry("glass").or_insert(json!(false));
    }

    fn contrast_warnings(&self) -> Vec<String> {
        let mut w = vec![];
        let mut pair = |what: &str, fg: Option<String>, bg: Option<String>| {
            if let (Some(f), Some(b)) = (fg, bg) {
                if let Some(c) = contrast(&f, &b) {
                    if c < 4.5 {
                        w.push(format!("{what}: contrast {c:.1}:1 is below 4.5:1"));
                    }
                }
            }
        };
        pair("text on background", self.color("text"), self.color("background"));
        pair("text on surface", self.color("text"), self.color("surface"));
        pair("on_accent on accent", self.color("on_accent"), self.color("accent"));
        for p in PANELS {
            let fill = self.panel(p, "fill").and_then(|v| v.as_str().map(String::from));
            let text = self.panel(p, "text").and_then(|v| v.as_str().map(String::from)).or_else(|| self.color("text"));
            if fill.is_some() {
                pair(&format!("{p} text"), text, fill);
            }
        }
        w
    }

    /// CSS custom properties for every token (`--ht-accent: #…`) and panel
    /// classes (`.ht-card`, `.ht-button`…), for pages.
    pub fn css(&self) -> String {
        let mut s = String::from(":root {\n");
        for (k, v) in &self.tokens {
            let val = match v {
                Json::String(x) => x.clone(),
                Json::Number(n) if NUMBER_TOKENS.contains(&k.as_str()) && k != "font_weight" => format!("{}px", num(n)),
                Json::Number(n) => num(n),
                Json::Bool(b) => b.to_string(),
                _ => continue,
            };
            s.push_str(&format!("  --ht-{}: {};\n", k.replace('_', "-"), css_escape(&val)));
        }
        s.push_str("}\n");
        for p in PANELS {
            let g = |k: &str, token: &str, unit: &str| -> String {
                match self.panel(p, k) {
                    Some(Json::String(x)) => css_escape(&x),
                    Some(Json::Number(n)) => format!("{}{unit}", num(&n)),
                    _ => format!("var(--ht-{})", token.replace('_', "-")),
                }
            };
            let fill_default = match *p {
                "button" => "accent",
                "field" | "card" | "popup" => "surface_alt",
                _ => "surface",
            };
            let text_default = if *p == "button" { "on_accent" } else { "text" };
            s.push_str(&format!(
                ".ht-{p} {{ background: {}; color: {}; border: {} solid {}; border-radius: {}; box-shadow: {} {} var(--ht-shadow-blur) var(--ht-shadow); }}\n",
                g("fill", fill_default, ""),
                g("text", text_default, ""),
                g("border_width", "border_width", "px"),
                g("border", "border", ""),
                g("radius", "radius", "px"),
                g("shadow_x", "shadow_x", "px"),
                g("shadow_y", "shadow_y", "px"),
            ));
        }
        s
    }
}

/// `6` rather than `6.0`.
fn num(n: &serde_json::Number) -> String {
    match n.as_f64() {
        Some(f) if f.fract() == 0.0 && f.abs() < 1e12 => format!("{}", f as i64),
        Some(f) => format!("{f}"),
        None => n.to_string(),
    }
}

fn css_escape(v: &str) -> String {
    v.chars().filter(|c| !matches!(c, ';' | '{' | '}' | '<' | '>' | '\\')).collect()
}

// ── Theme programs ─────────────────────────────────────────────────────

fn zsp_to_json(v: &stonedgem_zsharp::Value) -> Json {
    use stonedgem_zsharp::Value as V;
    match v {
        V::Null => Json::Null,
        V::Bool(b) => json!(b),
        V::Int(i) => json!(i),
        V::Float(f) => json!(f),
        V::Str(s) => json!(s.to_string()),
        V::Color(c) => json!(format!("#{:06x}", c >> 8)),
        V::List(l) => Json::Array(l.borrow().iter().map(zsp_to_json).collect()),
        V::Map(m) => Json::Object(m.borrow().iter().map(|(k, v)| (k.clone(), zsp_to_json(v))).collect()),
        other => json!(other.to_string()),
    }
}

pub fn zxx_to_json(v: &zsharpend::Value) -> Json {
    use zsharpend::Value as V;
    match v {
        V::Null => Json::Null,
        V::Bool(b) => json!(b),
        V::Int(i) => json!(i),
        V::Float(f) => json!(f),
        V::Str(s) => json!(s.to_string()),
        V::Color(c) => json!(format!("#{:06x}", c >> 8)),
        V::List(l) => Json::Array(l.borrow().iter().map(zxx_to_json).collect()),
        V::Map(m) => Json::Object(m.borrow().iter().map(|(k, v)| (k.clone(), zxx_to_json(v))).collect()),
        V::Struct(s) => Json::Object(s.borrow().fields.iter().map(|(k, v)| (k.clone(), zxx_to_json(v))).collect()),
        other => json!(other.to_string()),
    }
}

pub fn json_to_zxx(j: &Json) -> zsharpend::Value {
    use zsharpend::Value as V;
    match j {
        Json::Null => V::Null,
        Json::Bool(b) => V::Bool(*b),
        Json::Number(n) => match n.as_i64() {
            Some(i) => V::Int(i),
            None => V::Float(n.as_f64().unwrap_or(0.0)),
        },
        Json::String(s) => V::str(s),
        Json::Array(a) => V::list(a.iter().map(json_to_zxx).collect()),
        Json::Object(o) => V::map(o.iter().map(|(k, v)| (k.clone(), json_to_zxx(v))).collect()),
    }
}

fn run_zsp(src: &str, dark: bool) -> Result<Json, String> {
    let mut p = stonedgem_zsharp::Program::from_source(src)?;
    p.interp.limits.time = std::time::Duration::from_millis(500);
    match p.interp.call_global("tokens", vec![stonedgem_zsharp::Value::Bool(dark)])? {
        Some(v @ stonedgem_zsharp::Value::Map(_)) => Ok(zsp_to_json(&v)),
        Some(other) => Err(format!("tokens() returned {}, not a map", other.type_name())),
        None => Err("needs `func tokens(dark)`".into()),
    }
}

fn run_zxx(src: &str, dark: bool) -> Result<Json, String> {
    let mut p = zsharpend::Program::from_zxx(src)?;
    p.interp.limits.time = std::time::Duration::from_millis(500);
    match p.interp.call_global("tokens", vec![zsharpend::Value::Bool(dark)])? {
        Some(v @ zsharpend::Value::Map(_)) => Ok(zxx_to_json(&v)),
        Some(other) => Err(format!("tokens() returned {}, not a map", other.type_name())),
        None => Err("needs `fn tokens(dark)`".into()),
    }
}

// ── Start page and shader ──────────────────────────────────────────────

/// The theme's background shader drawn at `w`×`h`, as PNG.
pub fn background_png(h: &Hypertheme, w: u32, ht: u32) -> Option<Result<Vec<u8>, String>> {
    let src = h.files.get("background.zsg")?;
    Some((|| {
        let m = stonedgem_zsharp::zsg::Module::parse(src)?;
        let c = stonedgem_zsharp::canvas::Canvas::new(w as i64, ht as i64)?;
        c.shade(&m, "background", (0, 0, w as i32, ht as i32), &Default::default())?;
        c.png()
    })())
}

/// The start page's HTML for this theme: its CSS (tokens, panels,
/// `pages.css`), its TSX start page if it has one (otherwise a default
/// built from the panel classes), and its shader background.
pub fn start_page(h: &Hypertheme, r: &Resolved, product: &str, background_data_url: Option<&str>) -> Result<String, String> {
    let esc = |s: &str| s.replace('&', "&amp;").replace('<', "&lt;");
    let mut css = r.css();
    css.push_str(&format!(
        "html,body{{margin:0;min-height:100vh;background:var(--ht-background) {} center/cover no-repeat fixed;color:var(--ht-text);font-family:{};font-weight:var(--ht-font-weight)}}\n",
        background_data_url.map(|u| format!("url({u})")).unwrap_or_default(),
        r.tokens.get("font_family").and_then(Json::as_str).map(|f| format!("{}, system-ui, sans-serif", css_escape(f))).unwrap_or_else(|| "system-ui, sans-serif".into())
    ));
    css.push_str(".ht-start{max-width:880px;margin:0 auto;padding:64px 32px}.ht-start h1{font-size:44px;margin:0 0 24px}.ht-row{display:flex;gap:16px;flex-wrap:wrap}.ht-card{padding:18px 20px;flex:1;min-width:200px}.ht-button{display:inline-block;padding:10px 18px;font-weight:700;text-decoration:none}.ht-field{padding:12px 16px;margin:0 0 24px;font-size:18px}\n");
    if let Some(p) = h.files.get("pages.css") {
        css.push_str(&css_escape_block(p));
    }
    let body = match h.files.get("start.tsx") {
        Some(tsx) => {
            let js = stonedgem_extensions::tscompile::transpile_tsx(tsx, "start.tsx")?;
            let mut code = String::new();
            for line in js.lines() {
                let t = line.trim_start();
                if let Some(rest) = t.strip_prefix("export default ") {
                    code.push_str("__oynixWidget = ");
                    code.push_str(rest);
                } else if let Some(rest) = t.strip_prefix("export ") {
                    if !rest.starts_with('{') {
                        code.push_str(rest);
                    }
                } else {
                    code.push_str(line);
                }
                code.push('\n');
            }
            let tokens = serde_json::to_string(&r.tokens).unwrap_or_else(|_| "{}".into());
            format!(
                "<div id=\"root\"></div><script>\nvar oynix = Object.freeze({{ id: \"hypertheme\", data: [] }});\nvar theme = Object.freeze({{ name: {}, product: {}, tokens: {tokens} }});\nvar __oynixWidget = null;\n{}\n{}\nif (typeof __oynixWidget === 'function') render(h(__oynixWidget, null));\n</script>",
                serde_json::to_string(&h.manifest.name).unwrap_or_default(),
                serde_json::to_string(product).unwrap_or_default(),
                stonedgem_extensions::registry::WIDGET_RUNTIME,
                code.replace("</", "<\\/")
            )
        }
        None => format!(
            "<main class=\"ht-start\"><h1>{}</h1><div class=\"ht-field\">Search or type an address</div><div class=\"ht-row\"><div class=\"ht-card\"><b>Theme</b><br>{}</div><div class=\"ht-card\"><b>Made by</b><br>{}</div></div></main>",
            esc(product),
            esc(&h.manifest.name),
            esc(if h.manifest.author.is_empty() { "—" } else { &h.manifest.author })
        ),
    };
    Ok(format!("<!doctype html><html><head><meta charset=\"utf-8\"><title>New tab</title><style>{css}</style></head><body>{body}</body></html>"))
}

fn css_escape_block(css: &str) -> String {
    // Keep a theme's CSS inside its <style>.
    css.replace("</", "<\\/")
}

// ── Files ──────────────────────────────────────────────────────────────

fn read_dir(dir: &Path) -> Result<BTreeMap<String, String>, String> {
    let mut out = BTreeMap::new();
    for f in FILES {
        let p = dir.join(f);
        if p.is_file() {
            out.insert(f.to_string(), std::fs::read_to_string(&p).map_err(|e| format!("{f}: {e}"))?);
        }
    }
    Ok(out)
}

fn read_zip(bytes: &[u8]) -> Result<BTreeMap<String, String>, String> {
    let mut z = zip::ZipArchive::new(std::io::Cursor::new(bytes)).map_err(|e| format!("not a .hypertheme (zip) file: {e}"))?;
    let mut out = BTreeMap::new();
    for i in 0..z.len() {
        let mut f = z.by_index(i).map_err(|e| e.to_string())?;
        if f.is_dir() {
            continue;
        }
        let name = f.name().trim_start_matches("./").to_string();
        if f.size() as usize > MAX_FILE {
            return Err(format!("{name} is larger than 1 MB"));
        }
        let mut s = String::new();
        f.by_ref().take(MAX_FILE as u64 + 1).read_to_string(&mut s).map_err(|e| format!("{name}: {e}"))?;
        out.insert(name, s);
    }
    Ok(out)
}

pub fn write_zip(path: &Path, files: &BTreeMap<String, String>) -> Result<(), String> {
    let mut buf = std::io::Cursor::new(Vec::new());
    {
        let mut w = zip::ZipWriter::new(&mut buf);
        let opts = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
        for (name, text) in files {
            w.start_file(name.as_str(), opts).map_err(|e| e.to_string())?;
            w.write_all(text.as_bytes()).map_err(|e| e.to_string())?;
        }
        w.finish().map_err(|e| e.to_string())?;
    }
    if let Some(d) = path.parent() {
        std::fs::create_dir_all(d).map_err(|e| e.to_string())?;
    }
    std::fs::write(path, buf.into_inner()).map_err(|e| e.to_string())
}

/// Where installed hyperthemes live (in the profile).
pub fn dir() -> std::path::PathBuf {
    stonedgem_profile::data_dir().join("hyperthemes")
}

/// Install a `.hypertheme` file or folder; returns its file name in [`dir`].
pub fn install(src: &Path) -> Result<String, String> {
    let h = Hypertheme::open(src)?;
    let slug: String = h.manifest.name.to_lowercase().chars().map(|c| if c.is_ascii_alphanumeric() { c } else { '-' }).collect::<String>().trim_matches('-').to_string();
    let file = format!("{}.hypertheme", if slug.is_empty() { "theme".into() } else { slug });
    h.write(&dir().join(&file))?;
    Ok(file)
}

/// Installed hyperthemes: (file name, name).
pub fn installed() -> Vec<(String, String)> {
    let Ok(rd) = std::fs::read_dir(dir()) else { return vec![] };
    let mut v: Vec<(String, String)> = rd
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().is_some_and(|x| x == "hypertheme"))
        .filter_map(|e| Hypertheme::open(&e.path()).ok().map(|h| (e.file_name().to_string_lossy().to_string(), h.manifest.name)))
        .collect();
    v.sort_by(|a, b| a.1.cmp(&b.1));
    v
}
