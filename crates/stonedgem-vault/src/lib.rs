//! Saved passwords and importing from Firefox, for StonedGem browsers.
//!
//! * [`passwords`]: the encrypted login store (AES-256-GCM, key file `0600`),
//!   with save/update rules and CSV import (Firefox, Chrome, Bitwarden,
//!   1Password exports).
//! * [`firefox`]: reads a Firefox profile's logins (`logins.json` +
//!   `key4.db`) and bookmarks (`places.sqlite`).
//!
//! Nothing here touches the network or a UI toolkit.

pub mod firefox;
pub mod passwords;
