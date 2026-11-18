//! The window's settings, kept in the profile's config folder as
//! `browser.json`.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Settings {
    /// Search engine id (`stonedgem_url::ENGINES`).
    pub search: String,
    /// The page the Home button and new windows open; empty: the start page.
    pub home: String,
    pub dark: bool,
    /// Accent colour the palette is made from, `#rrggbb`.
    pub accent: String,
    pub blocker: bool,
    /// Element hiding (cosmetic filtering).
    pub hide_elements: bool,
    /// Reopen the last session's tabs on start.
    pub restore_tabs: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            search: crate::SEARCH.into(),
            home: String::new(),
            dark: true,
            accent: "#2f7cf6".into(),
            blocker: true,
            hide_elements: true,
            restore_tabs: true,
        }
    }
}

/// Accent colours offered in Settings.
pub const ACCENTS: &[(&str, &str)] =
    &[("Blue", "#2f7cf6"), ("Teal", "#00a6a6"), ("Green", "#3a9d4a"), ("Amber", "#e0a100"), ("Rose", "#d6456b"), ("Violet", "#7c4dff")];

fn file() -> PathBuf {
    stonedgem_profile::config_dir().join("browser.json")
}

impl Settings {
    pub fn load() -> Self {
        std::fs::read(file()).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default()
    }

    pub fn save(&self) -> std::io::Result<()> {
        let b = serde_json::to_vec_pretty(self).map_err(std::io::Error::other)?;
        stonedgem_profile::write_atomic(&file(), &b)
    }

    /// Apply the blocker switches.
    pub fn apply_blocker(&self) {
        stonedgem_blocker::blocker::configure(self.blocker, true, true, self.hide_elements);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_fields_take_defaults() {
        let s: Settings = serde_json::from_str(r#"{"search": "kagi"}"#).unwrap();
        assert_eq!(s.search, "kagi");
        assert!(s.dark && s.blocker);
        assert_eq!(s.accent, "#2f7cf6");
    }
}
