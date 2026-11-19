//! Hyperthemes in the window: which one is in use, the palette and start
//! page made from it, and Theme Studio (the built-in power extension that
//! makes them).

use stonedgem_extensions::registry::{self, Entry};
use stonedgem_hypertheme::{Hypertheme, Resolved};

use crate::palette::Palette;
use crate::settings::Settings;

/// The theme in use, resolved for the window's mode.
pub struct InUse {
    pub file: String,
    pub theme: Hypertheme,
    pub resolved: Resolved,
}

/// The window's palette from the settings: the hypertheme's when one is
/// chosen (and loads), the accent palette otherwise. The error says why a
/// chosen theme didn't load.
pub fn palette(s: &Settings) -> (Palette, Option<InUse>, Option<String>) {
    if s.hypertheme.is_empty() {
        return (Palette::new(&s.accent, s.dark), None, None);
    }
    match stonedgem_hypertheme::load(&s.hypertheme) {
        Ok(theme) => {
            let resolved = theme.resolve(s.dark);
            let p = Palette::from_hypertheme(&resolved);
            (p, Some(InUse { file: s.hypertheme.clone(), theme, resolved }), None)
        }
        Err(e) => (Palette::new(&s.accent, s.dark), None, Some(format!("{}: {e}", s.hypertheme))),
    }
}

/// The start page for a hypertheme, written to the cache; its `file://` URL.
pub fn start_page(t: &InUse, product: &str) -> Result<String, String> {
    let bg = match stonedgem_hypertheme::background_png(&t.theme, 1280, 720) {
        Some(Ok(png)) => Some(format!("data:image/png;base64,{}", crate::zs_hooks::base64_encode(&png))),
        _ => None,
    };
    let html = stonedgem_hypertheme::start_page(&t.theme, &t.resolved, product, bg.as_deref())?;
    let dir = stonedgem_profile::cache_dir().join("start");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let file = dir.join(format!("hypertheme-{}.html", if t.resolved.dark { "dark" } else { "light" }));
    stonedgem_profile::write_atomic(&file, html.as_bytes()).map_err(|e| e.to_string())?;
    Ok(format!("file://{}", file.display()))
}

/// Whether `url` is a hypertheme start page (the address bar shows it empty).
pub fn is_start_page(url: &str) -> bool {
    url.starts_with("file://") && url.contains("/start/hypertheme-") && url.ends_with(".html")
}

pub const STUDIO_NAME: &str = "Theme Studio";

/// Install Theme Studio (or update it when this build has a newer one).
/// Returns its extension id.
pub fn install_studio(list: &mut Vec<Entry>) -> Result<String, String> {
    let version = stonedgem_hypertheme::studio_version();
    let files = stonedgem_hypertheme::studio_files();
    // Up to date when the version matches and its page and script are this build's.
    let current = |e: &Entry| {
        e.version == version
            && files.iter().filter(|(f, _)| matches!(*f, "extra.html" | "script.zxx")).all(|(f, text)| std::fs::read_to_string(std::path::Path::new(&e.path).join(f)).is_ok_and(|t| &t == text))
    };
    if let Some(e) = list.iter().find(|e| e.builtin && e.name == STUDIO_NAME && current(e)) {
        return Ok(e.id.clone());
    }
    let src = stonedgem_profile::cache_dir().join("theme-studio-src");
    let _ = std::fs::remove_dir_all(&src);
    std::fs::create_dir_all(&src).map_err(|e| e.to_string())?;
    for (f, text) in files {
        std::fs::write(src.join(f), text).map_err(|e| format!("{f}: {e}"))?;
    }
    let e = registry::install_builtin(&src, list);
    let _ = std::fs::remove_dir_all(&src);
    let e = e?;
    registry::save(list);
    Ok(e.id)
}

/// The `chrome.*` bridge for an extension's own page (`extra.html`), so it
/// can message its script.
pub fn page_bridge(e: &Entry) -> String {
    let manifest = std::fs::read_to_string(std::path::Path::new(&e.path).join("oynix.json"))
        .ok()
        .and_then(|t| serde_json::from_str::<serde_json::Value>(&t).ok())
        .unwrap_or_else(|| serde_json::json!({ "name": e.name, "version": e.version }));
    crate::bridge::chrome_shim(&e.id, &manifest)
}
