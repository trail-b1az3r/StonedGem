//! What the browser using this crate supplies: its name, whom it trusts for
//! `.poweroynix` packages and, if it has a QML runtime, a checker for QML
//! panels (`graphics.qt`, `theme-shape.qml`).

use std::path::Path;
use std::sync::OnceLock;

/// Checks QML source; `Err` holds the message shown to the user.
pub type QmlCheck = fn(&str) -> Result<(), String>;

/// The product's side of extension handling. Set once at start-up with [`set`].
#[derive(Clone, Copy)]
pub struct Host {
    /// Shown as who verified built-in power extensions, e.g. `"OyNIx"`.
    pub name: &'static str,
    /// Trusted signing keys for `.poweroynix`, one per line
    /// (see `stonedgem-poweroynix`). Empty: no signed package verifies.
    pub trusted_keys: &'static str,
    /// Checks a QML panel's source. `None`: the browser has no QML runtime,
    /// and packages with QML panels are refused.
    pub check_qml: Option<QmlCheck>,
    /// The engine's page↔browser channel script (Qt's `qwebchannel.js`),
    /// put in front of scripts that talk to the browser. `None`: none.
    pub webchannel_js: Option<fn() -> String>,
}

const DEFAULT: Host = Host { name: "StonedGem", trusted_keys: "", check_qml: None, webchannel_js: None };

static HOST: OnceLock<Host> = OnceLock::new();

/// Set the host. Only the first call counts.
pub fn set(h: Host) {
    let _ = HOST.set(h);
}

fn get() -> &'static Host {
    HOST.get().unwrap_or(&DEFAULT)
}

pub(crate) fn name() -> &'static str {
    get().name
}

pub(crate) fn verify_power(dir: &Path) -> Result<stonedgem_poweroynix::Verified, String> {
    stonedgem_poweroynix::verify_info(dir, get().trusted_keys)
}

pub(crate) fn check_qml(qml: &str) -> Result<(), String> {
    match get().check_qml {
        Some(f) => f(qml),
        None => Err("QML panels (graphics.qt, theme-shape.qml) need a browser with a QML runtime".into()),
    }
}

pub(crate) fn webchannel_js() -> String {
    get().webchannel_js.map(|f| f()).unwrap_or_default()
}

/// Checks a Slint panel (`panel.slint`) in the extension folder given.
pub type SlintCheck = fn(&str, &Path) -> Result<(), String>;

static SLINT: OnceLock<SlintCheck> = OnceLock::new();

/// A browser that shows Slint panels sets how to check them (once).
pub fn set_slint_check(f: SlintCheck) {
    let _ = SLINT.set(f);
}

pub(crate) fn has_qml() -> bool {
    get().check_qml.is_some()
}

/// Without a checker, `panel.slint` is kept unchecked (the browser doesn't show it).
pub(crate) fn check_slint(src: &str, dir: &Path) -> Result<(), String> {
    match SLINT.get() {
        Some(f) => f(src, dir).map_err(|e| format!("panel.slint: {e}")),
        None => Ok(()),
    }
}
