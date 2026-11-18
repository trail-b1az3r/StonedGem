//! Extension packages for StonedGem browsers, independent of the engine.
//!
//! * [`registry`]: installing, converting and listing extensions — Chrome
//!   and Firefox WebExtensions (MV2/MV3, `.crx`, `.xpi`), the native
//!   `.oynix` format (`oynix.json` + `code.ts` + `content.css`) and verified
//!   `.poweroynix` packages — plus the page scripts, widgets and network.ts
//!   pages built from them.
//! * [`tscompile`]: TypeScript and TSX to JavaScript (oxc).
//! * [`material`]: Material colour schemes, used for extension themes.
//!
//! The product calls [`host::set`] at start-up with its name, trusted keys
//! and QML checker, and sets the profile folders (`stonedgem-profile`).

pub mod host;
pub mod matching;
pub use stonedgem_theme::material;
pub mod registry;
pub mod tscompile;

pub use host::Host;
