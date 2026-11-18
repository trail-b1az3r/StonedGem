//! The open tabs, saved so the next start reopens them
//! (`session.json` in the profile's data folder).

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Session {
    /// Addresses of the open tabs, web or internal (`stonedgem://…`).
    pub tabs: Vec<String>,
    pub active: usize,
}

fn file() -> std::path::PathBuf {
    stonedgem_profile::data_dir().join("session.json")
}

impl Session {
    pub fn load() -> Option<Session> {
        let s: Session = serde_json::from_slice(&std::fs::read(file()).ok()?).ok()?;
        (!s.tabs.is_empty()).then_some(s)
    }

    pub fn save(&self) {
        if let Ok(b) = serde_json::to_vec(self) {
            let _ = stonedgem_profile::write_atomic(&file(), &b);
        }
    }

    /// Only addresses worth reopening: web pages and internal pages, not
    /// the blank start page or `data:` URLs.
    pub fn keep(url: &str) -> bool {
        url.starts_with("https://") || url.starts_with("http://") || url.starts_with("stonedgem://")
    }
}
