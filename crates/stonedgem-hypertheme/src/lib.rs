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
//! | `iced.toml` | the Iced window's panels: `[toolbar] fill = "#…" border_width = 3 …` (`[dark.toolbar]` for dark mode) |
//! | `pages.css` | CSS for the start page and extension pages (after the generated `--ht-*` variables) |
//! | `start.tsx` | a **TSX** start page (`export default function Start() { … }`) |
//! | `chrome.qml` | **QML** for OyNIx (a `Theme` override) |
//! | `background.zsg` | a fragment shader (`fragment background(uv: vec2) -> color`) drawn behind the start page |
//! | `graph.json` | the Theme Studio graph it was made from, to edit it again |
//! | `modules/*.zxx` | Zsharpend modules the theme's programs `import` |
//! | `include/*.zsx` | Z#++ headers the theme's programs `#include` / `import` |
//! | `images/*` | pictures (PNG, JPEG, WebP, GIF, SVG): background images and other art |
//!
//! Tokens are merged in that order — `tokens.json`, `style.rs`,
//! `theme.zsp`, `theme.zxx`, `iced.toml` — later ones winning, then the
//! `dark` overrides when the browser is dark. Theme programs run in the
//! Z#++/Zsharpend sandboxes and can only read the theme's own files.
//!
//! ## Power hyperthemes (`.phypertheme`)
//!
//! A theme whose `hypertheme.toml` says `power = true` may also hold parts
//! that run code in the browser's interface, so installing one is a choice
//! the user confirms (like a power extension):
//!
//! | File | What it gives |
//! |---|---|
//! | `global.zxx` | a **Zsharpend** program: `fn global(dark)` returns a map of global settings (glass, background image, overlay, motion, effect parameters…) and may also set tokens (`"tokens"`) and panels |
//! | `global.zsg` | a fragment shader (`fragment global(uv: vec2) -> color`) laid over the whole window (its alpha decides how much shows) |
//! | `global.qml` | **QML** layers over (or under) the whole interface: glass, frames, particles… |
//! | `effect.qml` | a **QML** layer effect applied to the whole interface (it receives the rendered UI as `source`): colour grading, blur, glow, distortion… |
//! | `format-panel.qt` | **QML** (`QtObject { property int toolbarHeight: … }`) setting the interface's format: bar and tab heights, font sizes, spacing, corner scale |
//! | `styles.tsx` | **TSX** exporting per-component styles (`export const button = { radius: 4, weight: 700 }`) and CSS for pages |

use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::path::Path;

use serde::Deserialize;
use serde_json::{json, Map, Value as Json};

pub mod basic_rust;
pub mod studio;

/// The files a hypertheme may hold (besides `modules/`, `include/` and `images/`).
pub const FILES: &[&str] = &[
    "hypertheme.toml", "tokens.json", "style.rs", "theme.zsp", "theme.zxx", "iced.toml", "pages.css", "start.tsx", "chrome.qml", "background.zsg", "graph.json",
];
/// The parts only a power hypertheme (`.phypertheme`) may hold.
pub const POWER_FILES: &[&str] = &["global.zxx", "global.zsg", "global.qml", "effect.qml", "format-panel.qt", "styles.tsx"];
/// Picture types allowed under `images/`.
pub const IMAGE_TYPES: &[&str] = &["png", "jpg", "jpeg", "webp", "gif", "svg"];
const MAX_FILE: usize = 1 << 20;
const MAX_TOTAL: usize = 4 << 20;
const MAX_IMAGE: usize = 8 << 20;
const MAX_IMAGES: usize = 24 << 20;

/// What kind of file `name` is in a hypertheme, or why it can't be one.
fn kind_of(name: &str) -> Result<Part, String> {
    if FILES.contains(&name) {
        return Ok(Part::Text);
    }
    if POWER_FILES.contains(&name) {
        return Ok(Part::Power);
    }
    let safe = |rest: &str| !rest.is_empty() && !rest.contains('/') && !rest.starts_with('.') && rest.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_'));
    if let Some(rest) = name.strip_prefix("modules/") {
        if safe(rest) && rest.ends_with(".zxx") {
            return Ok(Part::Text);
        }
    }
    if let Some(rest) = name.strip_prefix("include/") {
        if safe(rest) && rest.ends_with(".zsx") {
            return Ok(Part::Text);
        }
    }
    if let Some(rest) = name.strip_prefix("images/") {
        let ext = rest.rsplit('.').next().unwrap_or("").to_ascii_lowercase();
        if safe(rest) && IMAGE_TYPES.contains(&ext.as_str()) {
            return Ok(Part::Image);
        }
    }
    Err(format!(
        "a hypertheme can't hold `{name}` (it holds {}, modules/*.zxx, include/*.zsx and images/*; a .phypertheme also {})",
        FILES.join(", "),
        POWER_FILES.join(", ")
    ))
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum Part {
    Text,
    Power,
    Image,
}

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
    /// A power hypertheme (`.phypertheme`): may hold parts that run code in
    /// the interface (global.qml, effect.qml, …).
    #[serde(default)]
    pub power: bool,
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

/// A loaded hypertheme: its manifest, its text files and its pictures.
#[derive(Clone, Debug)]
pub struct Hypertheme {
    pub manifest: Manifest,
    pub files: BTreeMap<String, String>,
    /// `images/*`, by path.
    pub assets: BTreeMap<String, Vec<u8>>,
}

/// Tokens for one mode, ready to use.
#[derive(Clone, Debug, Default)]
pub struct Resolved {
    pub dark: bool,
    pub tokens: Map<String, Json>,
    pub panels: BTreeMap<String, Map<String, Json>>,
    /// Problems worth showing (contrast too low, a theme program failed…).
    pub warnings: Vec<String>,
    /// Power hyperthemes: what `global.zxx`'s `global(dark)` returned.
    pub global: Map<String, Json>,
}

impl Hypertheme {
    /// From a `.hypertheme` file (zip) or a folder.
    pub fn open(path: &Path) -> Result<Hypertheme, String> {
        let (files, assets) = if path.is_dir() { read_dir(path)? } else { read_zip(&std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?)? };
        Hypertheme::from_parts(files, assets)
    }

    /// A theme of text files only.
    pub fn from_files(files: BTreeMap<String, String>) -> Result<Hypertheme, String> {
        Hypertheme::from_parts(files, BTreeMap::new())
    }

    pub fn from_parts(files: BTreeMap<String, String>, assets: BTreeMap<String, Vec<u8>>) -> Result<Hypertheme, String> {
        let mut total = 0;
        let mut power_parts = vec![];
        for (name, text) in &files {
            match kind_of(name)? {
                Part::Image => return Err(format!("{name} is a picture, not text")),
                Part::Power => power_parts.push(name.clone()),
                Part::Text => {}
            }
            if text.len() > MAX_FILE {
                return Err(format!("{name} is larger than 1 MB"));
            }
            total += text.len();
        }
        if total > MAX_TOTAL {
            return Err("a hypertheme's text is at most 4 MB".into());
        }
        let mut pictures = 0;
        for (name, bytes) in &assets {
            if kind_of(name)? != Part::Image {
                return Err(format!("{name} isn't a picture under images/"));
            }
            if bytes.len() > MAX_IMAGE {
                return Err(format!("{name} is larger than 8 MB"));
            }
            if !looks_like_image(bytes) {
                return Err(format!("{name} isn't a PNG, JPEG, WebP, GIF or SVG picture"));
            }
            pictures += bytes.len();
        }
        if pictures > MAX_IMAGES {
            return Err("a hypertheme's pictures are at most 24 MB together".into());
        }
        let m = files.get("hypertheme.toml").ok_or("a hypertheme needs hypertheme.toml")?;
        let manifest = toml::from_str::<ManifestFile>(m).map_err(|e| format!("hypertheme.toml: {e}"))?.hypertheme;
        if manifest.name.trim().is_empty() {
            return Err("hypertheme.toml: `name` can't be empty".into());
        }
        if !power_parts.is_empty() && !manifest.power {
            return Err(format!("{} can only be in a power hypertheme: add `power = true` to hypertheme.toml (and name it .phypertheme)", power_parts.join(", ")));
        }
        let h = Hypertheme { manifest, files, assets };
        h.check()?;
        Ok(h)
    }

    /// A power hypertheme (`.phypertheme`).
    pub fn is_power(&self) -> bool {
        self.manifest.power
    }

    /// The file name it installs as: `<slug>.hypertheme` or `<slug>.phypertheme`.
    pub fn file_name(&self) -> String {
        let slug: String = self.manifest.name.to_lowercase().chars().map(|c| if c.is_ascii_alphanumeric() { c } else { '-' }).collect::<String>().trim_matches('-').to_string();
        format!("{}.{}", if slug.is_empty() { "theme".into() } else { slug }, if self.is_power() { "phypertheme" } else { "hypertheme" })
    }

    /// Read a theme file as the theme programs see it (`modules/`, `include/`…).
    fn reader(&self) -> impl Fn(&str) -> Option<String> + '_ {
        move |p: &str| {
            let p = p.trim_start_matches("./");
            // `import palette` looks for modules/palette/mod.zxx; a theme
            // keeps it as modules/palette.zxx.
            let flat = p.strip_prefix("modules/").and_then(|r| r.strip_suffix("/mod.zxx")).map(|n| format!("modules/{n}.zxx"));
            self.files.get(p).or_else(|| flat.and_then(|f| self.files.get(&f))).cloned()
        }
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
        for (name, t) in &self.files {
            let r = if name.ends_with(".zxx") {
                zsharpend::parser::parse(t, zsharpend::ast::Kind::Zxx).map(|_| ())
            } else if name.ends_with(".zsx") {
                zsharpend::parser::parse(t, zsharpend::ast::Kind::Header).map(|_| ())
            } else {
                Ok(())
            };
            r.map_err(|e| format!("{name}: {e}"))?;
        }
        if let Some(t) = self.files.get("global.zsg") {
            let m = stonedgem_zsharp::zsg::Module::parse(t).map_err(|e| format!("global.zsg: {e}"))?;
            if m.entry("global").is_none() {
                return Err("global.zsg: needs `fragment global(uv: vec2) -> color`".into());
            }
        }
        if let Some(t) = self.files.get("styles.tsx") {
            stonedgem_extensions::tscompile::transpile_tsx(t, "styles.tsx")?;
        }
        for name in ["global.qml", "effect.qml", "format-panel.qt", "chrome.qml"] {
            if let Some(t) = self.files.get(name) {
                // Qt checks QML when it loads it; here only that it looks like QML.
                if !t.lines().any(|l| l.trim_start().starts_with("import ")) || !t.contains('{') {
                    return Err(format!("{name}: doesn't look like QML (it needs `import QtQuick` and an object)"));
                }
            }
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
        if self.files.contains_key("theme.zsp") {
            match run_zsp(&self.reader(), dark) {
                Ok(v) => layers.push(("theme.zsp".into(), v)),
                Err(e) => r.warnings.push(format!("theme.zsp: {e}")),
            }
        }
        if self.files.contains_key("theme.zxx") {
            match run_zxx(&self.reader(), "theme.zxx", "tokens", dark) {
                Ok(v) => layers.push(("theme.zxx".into(), v)),
                Err(e) => r.warnings.push(format!("theme.zxx: {e}")),
            }
        }
        // global.zxx: global settings, and tokens/panels if it returns them.
        if self.is_power() && self.files.contains_key("global.zxx") {
            match run_zxx(&self.reader(), "global.zxx", "global", dark) {
                Ok(Json::Object(mut g)) => {
                    let mut layer = Map::new();
                    if let Some(Json::Object(t)) = g.remove("tokens") {
                        layer.extend(t);
                    }
                    if let Some(p) = g.remove("panels") {
                        layer.insert("panels".into(), p);
                    }
                    if !layer.is_empty() {
                        layers.push(("global.zxx".into(), Json::Object(layer)));
                    }
                    r.global = g;
                }
                Ok(other) => r.warnings.push(format!("global.zxx: global() returned {other}, not a map")),
                Err(e) => r.warnings.push(format!("global.zxx: {e}")),
            }
        }
        if let Some(t) = self.files.get("iced.toml") {
            if let Ok(tb) = toml::from_str::<toml::Table>(t) {
                // `[card]` styles a panel; `[dark.card]` styles it in dark mode.
                let mut v = serde_json::to_value(&tb).unwrap_or(Json::Null);
                let dark = v.as_object_mut().and_then(|o| o.remove("dark")).unwrap_or(json!({}));
                layers.push(("iced.toml".into(), json!({ "panels": v, "dark": { "panels": dark } })));
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

    /// Write as a `.hypertheme` / `.phypertheme` (zip).
    pub fn write(&self, path: &Path) -> Result<(), String> {
        write_zip_parts(path, &self.files, &self.assets)
    }

    /// Unpack every part into `dir` (so QML parts and pictures can be
    /// loaded from files). Anything else already in `dir` is removed.
    pub fn materialize(&self, dir: &Path) -> Result<(), String> {
        let _ = std::fs::remove_dir_all(dir);
        for (name, text) in &self.files {
            write_part(dir, name, text.as_bytes())?;
        }
        for (name, bytes) in &self.assets {
            write_part(dir, name, bytes)?;
        }
        Ok(())
    }

    /// `styles.tsx` as the body of a JavaScript function that returns its
    /// exports (`export const x = …` → `exports.x`, `export default` → `exports.default`).
    pub fn styles_js(&self) -> Option<Result<String, String>> {
        let tsx = self.files.get("styles.tsx")?;
        Some(stonedgem_extensions::tscompile::transpile_tsx(tsx, "styles.tsx").map(|js| {
            // Exports are assigned at the end, so a multi-line `export const`
            // stays whole.
            let mut out = String::from("var exports = {};\n");
            let mut names: Vec<String> = vec![];
            let ident = |r: &str| r.split(|c: char| !(c.is_alphanumeric() || c == '_' || c == '$')).next().unwrap_or("").to_string();
            for line in js.lines() {
                let t = line.trim_start();
                if let Some(rest) = t.strip_prefix("export default ") {
                    out.push_str("exports.default = ");
                    out.push_str(rest);
                } else if let Some((kw, rest)) = ["const ", "let ", "var "].iter().find_map(|k| t.strip_prefix("export ").and_then(|r| r.strip_prefix(k)).map(|r| (*k, r))) {
                    names.push(ident(rest));
                    out.push_str(kw);
                    out.push_str(rest);
                } else if let Some(rest) = t.strip_prefix("export function ") {
                    names.push(ident(rest));
                    out.push_str("function ");
                    out.push_str(rest);
                } else if t.starts_with("export {") || t.starts_with("import ") {
                    // Re-exports and imports aren't available in styles.tsx.
                } else {
                    out.push_str(line);
                }
                out.push('\n');
            }
            for n in names.iter().filter(|n| !n.is_empty()) {
                out.push_str(&format!("exports.{n} = {n};\n"));
            }
            out.push_str("return exports;\n");
            out
        }))
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

/// Run `theme.zsp`'s `tokens(dark)`; it can `#include` the theme's `include/*.zsx`.
fn run_zsp(read: &dyn Fn(&str) -> Option<String>, dark: bool) -> Result<Json, String> {
    let mut p = stonedgem_zsharp::Program::load("theme.zsp", read, |_| {})?;
    p.interp.limits.time = std::time::Duration::from_millis(500);
    match p.interp.call_global("tokens", vec![stonedgem_zsharp::Value::Bool(dark)])? {
        Some(v @ stonedgem_zsharp::Value::Map(_)) => Ok(zsp_to_json(&v)),
        Some(other) => Err(format!("tokens() returned {}, not a map", other.type_name())),
        None => Err("needs `func tokens(dark)`".into()),
    }
}

/// Run `entry`'s `func(dark)`; it can import the theme's `modules/*.zxx`
/// and `include/*.zsx`.
fn run_zxx(read: &dyn Fn(&str) -> Option<String>, entry: &str, func: &str, dark: bool) -> Result<Json, String> {
    let mut p = zsharpend::Program::load(entry, read, |_| {})?;
    p.interp.limits.time = std::time::Duration::from_millis(500);
    match p.interp.call_global(func, vec![zsharpend::Value::Bool(dark)])? {
        Some(v @ zsharpend::Value::Map(_)) => Ok(zxx_to_json(&v)),
        Some(other) => Err(format!("{func}() returned {}, not a map", other.type_name())),
        None => Err(format!("needs `fn {func}(dark)`")),
    }
}

// ── Start page and shader ──────────────────────────────────────────────

/// The theme's background shader drawn at `w`×`h`, as PNG.
pub fn background_png(h: &Hypertheme, w: u32, ht: u32) -> Option<Result<Vec<u8>, String>> {
    shade_png(h.files.get("background.zsg")?, "background", w, ht)
}

/// A power theme's `global.zsg` (laid over the whole window) at `w`×`h`, as PNG.
pub fn global_png(h: &Hypertheme, w: u32, ht: u32) -> Option<Result<Vec<u8>, String>> {
    if !h.is_power() {
        return None;
    }
    shade_png(h.files.get("global.zsg")?, "global", w, ht)
}

fn shade_png(src: &str, entry: &str, w: u32, ht: u32) -> Option<Result<Vec<u8>, String>> {
    Some((|| {
        let m = stonedgem_zsharp::zsg::Module::parse(src)?;
        let c = stonedgem_zsharp::canvas::Canvas::new(w.clamp(1, 4096) as i64, ht.clamp(1, 4096) as i64)?;
        c.shade(&m, entry, (0, 0, w.clamp(1, 4096) as i32, ht.clamp(1, 4096) as i32), &Default::default())?;
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

type Parts = (BTreeMap<String, String>, BTreeMap<String, Vec<u8>>);

fn looks_like_image(b: &[u8]) -> bool {
    b.starts_with(b"\x89PNG")
        || b.starts_with(b"GIF8")
        || b.starts_with(b"\xff\xd8\xff")
        || (b.starts_with(b"RIFF") && b.get(8..12) == Some(b"WEBP"))
        || String::from_utf8_lossy(&b[..b.len().min(512)]).to_ascii_lowercase().contains("<svg")
}

fn add_part(files: &mut BTreeMap<String, String>, assets: &mut BTreeMap<String, Vec<u8>>, name: String, bytes: Vec<u8>) -> Result<(), String> {
    if kind_of(&name)? == Part::Image {
        assets.insert(name, bytes);
    } else {
        let text = String::from_utf8(bytes).map_err(|_| format!("{name} isn't UTF-8 text"))?;
        files.insert(name, text);
    }
    Ok(())
}

fn read_dir(dir: &Path) -> Result<Parts, String> {
    let (mut files, mut assets) = (BTreeMap::new(), BTreeMap::new());
    for f in FILES.iter().chain(POWER_FILES) {
        let p = dir.join(f);
        if p.is_file() {
            add_part(&mut files, &mut assets, f.to_string(), std::fs::read(&p).map_err(|e| format!("{f}: {e}"))?)?;
        }
    }
    for sub in ["modules", "include", "images"] {
        let Ok(rd) = std::fs::read_dir(dir.join(sub)) else { continue };
        for e in rd.filter_map(|e| e.ok()) {
            let p = e.path();
            if !p.is_file() {
                continue;
            }
            let name = format!("{sub}/{}", e.file_name().to_string_lossy());
            let limit = if sub == "images" { MAX_IMAGE } else { MAX_FILE };
            if std::fs::metadata(&p).map(|m| m.len() as usize > limit).unwrap_or(false) {
                return Err(format!("{name} is too large"));
            }
            add_part(&mut files, &mut assets, name.clone(), std::fs::read(&p).map_err(|e| format!("{name}: {e}"))?)?;
        }
    }
    Ok((files, assets))
}

fn read_zip(bytes: &[u8]) -> Result<Parts, String> {
    let mut z = zip::ZipArchive::new(std::io::Cursor::new(bytes)).map_err(|e| format!("not a .hypertheme (zip) file: {e}"))?;
    let (mut files, mut assets) = (BTreeMap::new(), BTreeMap::new());
    for i in 0..z.len() {
        let mut f = z.by_index(i).map_err(|e| e.to_string())?;
        if f.is_dir() {
            continue;
        }
        let name = f.name().trim_start_matches("./").to_string();
        let limit = if name.starts_with("images/") { MAX_IMAGE } else { MAX_FILE };
        if f.size() as usize > limit {
            return Err(format!("{name} is too large"));
        }
        let mut b = Vec::new();
        f.by_ref().take(limit as u64 + 1).read_to_end(&mut b).map_err(|e| format!("{name}: {e}"))?;
        add_part(&mut files, &mut assets, name, b)?;
    }
    Ok((files, assets))
}

fn write_part(dir: &Path, name: &str, bytes: &[u8]) -> Result<(), String> {
    kind_of(name)?; // never outside the theme's own layout
    let p = dir.join(name);
    if let Some(d) = p.parent() {
        std::fs::create_dir_all(d).map_err(|e| e.to_string())?;
    }
    std::fs::write(&p, bytes).map_err(|e| format!("{name}: {e}"))
}

pub fn write_zip(path: &Path, files: &BTreeMap<String, String>) -> Result<(), String> {
    write_zip_parts(path, files, &BTreeMap::new())
}

pub fn write_zip_parts(path: &Path, files: &BTreeMap<String, String>, assets: &BTreeMap<String, Vec<u8>>) -> Result<(), String> {
    let mut buf = std::io::Cursor::new(Vec::new());
    {
        let mut w = zip::ZipWriter::new(&mut buf);
        let opts = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
        for (name, bytes) in files.iter().map(|(n, t)| (n, t.as_bytes())).chain(assets.iter().map(|(n, b)| (n, b.as_slice()))) {
            w.start_file(name.as_str(), opts).map_err(|e| e.to_string())?;
            w.write_all(bytes).map_err(|e| e.to_string())?;
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

/// Why an install stopped before writing anything.
pub const NEEDS_POWER_CONFIRM: &str = "power-hypertheme";

/// Install a `.hypertheme` file or folder; returns its file name in [`dir`].
/// A power hypertheme is refused (with an error starting with
/// [`NEEDS_POWER_CONFIRM`]) unless the user agreed: use [`install_with`].
pub fn install(src: &Path) -> Result<String, String> {
    install_with(src, false)
}

/// Install, allowing a power hypertheme when `allow_power` (the user
/// confirmed that its QML and programs may run in the interface).
pub fn install_with(src: &Path, allow_power: bool) -> Result<String, String> {
    let h = Hypertheme::open(src)?;
    if h.is_power() && !allow_power {
        return Err(format!("{NEEDS_POWER_CONFIRM}: “{}” is a power hypertheme: its QML and programs run in the browser's interface", h.manifest.name));
    }
    let file = h.file_name();
    h.write(&dir().join(&file))?;
    // A theme of the same name in the other kind is replaced.
    let other = if h.is_power() { file.replace(".phypertheme", ".hypertheme") } else { file.replace(".hypertheme", ".phypertheme") };
    let _ = std::fs::remove_file(dir().join(other));
    Ok(file)
}

/// Remove an installed theme. Built-in ones can't be removed.
pub fn uninstall(file: &str) -> Result<(), String> {
    check_name(file)?;
    std::fs::remove_file(dir().join(file)).map_err(|e| format!("{file}: {e}"))
}

fn check_name(file: &str) -> Result<(), String> {
    if file.is_empty() || file.contains(['/', '\\']) || file.starts_with('.') || !(file.ends_with(".hypertheme") || file.ends_with(".phypertheme")) {
        return Err(format!("“{file}” isn't a hypertheme file name"));
    }
    Ok(())
}

/// Installed hyperthemes: (file name, name).
pub fn installed() -> Vec<(String, String)> {
    let Ok(rd) = std::fs::read_dir(dir()) else { return vec![] };
    let mut v: Vec<(String, String)> = rd
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().is_some_and(|x| x == "hypertheme" || x == "phypertheme"))
        .filter_map(|e| Hypertheme::open(&e.path()).ok().map(|h| (e.file_name().to_string_lossy().to_string(), h.manifest.name)))
        .collect();
    v.sort_by(|a, b| a.1.cmp(&b.1));
    v
}

// ── Built-in hyperthemes ───────────────────────────────────────────────

/// Hyperthemes that ship with the browser, by file name. Neubrutalism was
/// made in Theme Studio: `graph.json` is its node graph, and every other
/// part is what the Studio's `script.zxx` generated from it.
pub fn builtins() -> Vec<(String, Hypertheme)> {
    macro_rules! theme {
        ($dir:literal, [$($f:literal),*]) => {{
            let files: BTreeMap<String, String> = [$(($f.to_string(), include_str!(concat!("../themes/", $dir, "/", $f)).to_string())),*].into_iter().collect();
            let h = Hypertheme::from_files(files).expect(concat!("built-in hypertheme ", $dir));
            (h.file_name(), h)
        }};
    }
    vec![
        theme!("neubrutalism", ["hypertheme.toml", "tokens.json", "style.rs", "theme.zxx", "theme.zsp", "iced.toml", "pages.css", "start.tsx", "chrome.qml", "background.zsg", "graph.json"]),
        theme!("liquid-glass", ["hypertheme.toml", "tokens.json", "global.zxx", "global.zsg", "global.qml", "format-panel.qt", "styles.tsx"]),
    ]
}

/// An installed or built-in hypertheme by file name (installed ones win).
pub fn load(file: &str) -> Result<Hypertheme, String> {
    check_name(file)?;
    let path = dir().join(file);
    if path.exists() {
        return Hypertheme::open(&path);
    }
    builtins().into_iter().find(|(f, _)| f == file).map(|(_, h)| h).ok_or(format!("no hypertheme “{file}”"))
}

/// Installed and built-in hyperthemes: (file name, name), built-ins first.
pub fn available() -> Vec<(String, String)> {
    let mine = installed();
    let mut v: Vec<(String, String)> = builtins().into_iter().filter(|(f, _)| !mine.iter().any(|(m, _)| m == f)).map(|(f, h)| (f, h.manifest.name)).collect();
    v.extend(mine);
    v
}

// ── Theme Studio ───────────────────────────────────────────────────────

/// The Theme Studio power extension's files, ready to install: its node
/// editor page, its Zsharpend script, and the Neubrutalism graph as the
/// page's template.
pub fn studio_files() -> Vec<(&'static str, String)> {
    let graph = include_str!("../themes/neubrutalism/graph.json").trim();
    vec![
        ("oynix.json", include_str!("../studio/oynix.json").to_string()),
        ("code.ts", include_str!("../studio/code.ts").to_string()),
        ("content.css", include_str!("../studio/content.css").to_string()),
        ("script.zxx", include_str!("../studio/script.zxx").to_string()),
        ("extra.html", include_str!("../studio/extra.html").replace("/*NEUBRUTALISM_GRAPH*/null", &graph.replace("</", "<\\/"))),
    ]
}

/// The Studio's version (from its `oynix.json`), to reinstall it when it changes.
pub fn studio_version() -> String {
    serde_json::from_str::<Json>(include_str!("../studio/oynix.json")).ok().and_then(|v| v["version"].as_str().map(String::from)).unwrap_or_default()
}
