//! Z#++ and Zsharpend in extensions. An OyNIx-format extension may include
//! `script.zsp` (Z#++) or `script.zxx` (Zsharpend, the next Z#++), with the
//! headers and GPU code it imports. It's loaded
//! from the extension's own folder only, in the Z#++ sandbox, and sees
//! just the extension API below; the browser emits `page_load` when a page
//! finishes loading:
//!
//! ```text
//! on page_load "*://*.example.com/*" (url, title) {
//!     inject_css("header { display: none }")
//!     notify("Tidied ${title}")
//! }
//! ```

use std::path::Path;

use stonedgem_zsharp::{Interp, Program, Value};

pub const SCRIPT: &str = "script.zsp";
/// A Zsharpend script (used instead of `script.zsp` when present).
pub const SCRIPT_ZXX: &str = "script.zxx";

/// Functions extension scripts may call (the browser provides them).
pub const API: &[(&str, &str)] = &[
    ("page_url", "page_url() -> string: the page's address"),
    ("page_title", "page_title() -> string: the page's title"),
    ("inject_css", "inject_css(css): add a style sheet to the page"),
    ("run_js", "run_js(code): run JavaScript in the page"),
    ("notify", "notify(text): show a short message in the browser"),
    ("store_get", "store_get(key) -> value: read this extension's saved value"),
    ("store_set", "store_set(key, value): save a value for this extension"),
];

/// Power extensions' Zsharpend scripts also get these (Theme Studio uses them).
pub const POWER_API: &[(&str, &str)] = &[
    ("to_json", "to_json(value) -> string: JSON text"),
    ("from_json", "from_json(text) -> value"),
    ("zsg_preview", "zsg_preview(code, w, h) -> string: a .zsg `fragment preview` drawn, as a data: URL"),
    ("hypertheme_save", "hypertheme_save(name, files) -> string: write a .hypertheme from a map of file → text; returns its file name"),
    ("hypertheme_apply", "hypertheme_apply(file): use an installed hypertheme"),
    ("hypertheme_resolve", "hypertheme_resolve(files, dark) -> map: the tokens, panels, CSS and warnings the browser would make of these parts"),
    ("hypertheme_list", "hypertheme_list() -> list: installed hyperthemes, [file, name]"),
];

pub fn has_script(dir: &Path) -> bool {
    dir.join(SCRIPT).is_file() || dir.join(SCRIPT_ZXX).is_file()
}

/// Whether the extension's script is Zsharpend (`script.zxx`).
pub fn is_zxx(dir: &Path) -> bool {
    dir.join(SCRIPT_ZXX).is_file()
}

fn reader(dir: &Path) -> impl Fn(&str) -> Option<String> {
    let root = dir.to_path_buf();
    move |p: &str| -> Option<String> {
        let full = root.join(p);
        if std::fs::symlink_metadata(&full).ok()?.file_type().is_symlink() {
            return None;
        }
        std::fs::read_to_string(full).ok()
    }
}

/// Load a Zsharpend `script.zxx` the same way (its own folder only).
pub fn load_zxx(dir: &Path, host: impl FnOnce(&mut zsharpend::Interp)) -> Result<zsharpend::Program, String> {
    zsharpend::Program::load(SCRIPT_ZXX, &reader(dir), host)
}

fn stub_zxx(it: &mut zsharpend::Interp) {
    for (name, _) in API.iter().chain(POWER_API) {
        it.register(name, |_, _| Ok(zsharpend::Value::Null));
    }
}

/// Load the extension's script, reading only files inside `dir`, after
/// `host` registers the API.
pub fn load(dir: &Path, host: impl FnOnce(&mut Interp)) -> Result<Program, String> {
    let root = dir.to_path_buf();
    let read = move |p: &str| -> Option<String> {
        let full = root.join(p);
        // `resolve` already refuses `..` above the root; also refuse links.
        if std::fs::symlink_metadata(&full).ok()?.file_type().is_symlink() {
            return None;
        }
        std::fs::read_to_string(full).ok()
    };
    Program::load(SCRIPT, &read, host)
}

/// Register no-op versions of the API (to check a script without a page).
pub fn stub_api(it: &mut Interp) {
    for (name, _) in API {
        it.register(name, |_, _| Ok(Value::Null));
    }
}

/// Load the script with the API stubbed: parse errors, header mismatches
/// and top-level errors show at install time.
pub fn check(dir: &Path) -> Result<(), String> {
    if is_zxx(dir) {
        return load_zxx(dir, stub_zxx).map(|_| ()).map_err(|e| format!("{SCRIPT_ZXX}: {e}"));
    }
    load(dir, stub_api).map(|_| ()).map_err(|e| format!("{SCRIPT}: {e}"))
}
