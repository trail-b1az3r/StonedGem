//! Z#++ in extensions. An OyNIx-format extension may include `script.zsp`
//! (and the `.zsx` headers and `.zsg` GPU code it imports). It's loaded
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

pub fn has_script(dir: &Path) -> bool {
    dir.join(SCRIPT).is_file()
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
    load(dir, stub_api).map(|_| ()).map_err(|e| format!("{SCRIPT}: {e}"))
}
