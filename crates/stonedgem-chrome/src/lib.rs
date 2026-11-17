//! The browser window's logic, kept apart from drawing so it can be tested
//! without a display: tabs, what the address bar does with typed text,
//! which keys go to the page, settings, colours and find-in-page.

use iced::keyboard::{key::Named, Key as IcedKey, Modifiers};
use stonedgem_engine::Key;

pub mod find;
pub mod palette;
pub mod settings;

/// The page a new tab opens.
pub const START_PAGE: &str = "data:text/html;charset=utf-8,<!DOCTYPE html><title>New tab</title>\
<style>body{font:16px sans-serif;background:%23101418;color:%23d8dee6;display:grid;place-items:center;height:90vh;margin:0}\
h1{font-weight:300;letter-spacing:.06em}p{color:%238a96a3}</style>\
<div><h1>StonedGem</h1><p>Type an address or a search in the bar above.</p></div>";

/// Search engine used for text that isn't an address, unless settings say otherwise.
pub const SEARCH: &str = "duckduckgo";

/// Pages the window draws itself (with Iced), not the engine.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Internal {
    History,
    Bookmarks,
    Settings,
}

impl Internal {
    pub const ALL: [Internal; 3] = [Internal::History, Internal::Bookmarks, Internal::Settings];

    pub fn address(self) -> &'static str {
        match self {
            Internal::History => "stonedgem://history",
            Internal::Bookmarks => "stonedgem://bookmarks",
            Internal::Settings => "stonedgem://settings",
        }
    }

    pub fn title(self) -> &'static str {
        match self {
            Internal::History => "History",
            Internal::Bookmarks => "Bookmarks",
            Internal::Settings => "Settings",
        }
    }

    /// The internal page typed text names, e.g. `stonedgem://history` or `about:settings`.
    pub fn from_address(text: &str) -> Option<Internal> {
        let t = text.trim().to_ascii_lowercase();
        let name = t.strip_prefix("stonedgem://").or_else(|| t.strip_prefix("about:"))?;
        Internal::ALL.into_iter().find(|i| i.title().eq_ignore_ascii_case(name.trim_end_matches('/')))
    }
}

/// What a tab shows in the tab strip.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TabInfo {
    pub title: String,
    pub url: String,
    pub loading: bool,
}

impl TabInfo {
    /// The tab strip label: the title, else the address, shortened.
    pub fn label(&self) -> String {
        let t = if self.title.trim().is_empty() { stonedgem_url::display(&self.url, false) } else { self.title.clone() };
        if t.chars().count() > 22 {
            t.chars().take(21).collect::<String>() + "…"
        } else {
            t
        }
    }
}

/// Typed address-bar text to the URL to load, searching with `engine`.
pub fn address_to_url(text: &str, engine: &str) -> String {
    stonedgem_url::resolve(text, engine, true, true)
}

/// Whether a page's visit belongs in history.
pub fn recordable(url: &str) -> bool {
    url.starts_with("https://") || url.starts_with("http://")
}

/// "just now", "5 min ago", "3 h ago", "2 days ago" for a UNIX time.
pub fn ago(then: i64, now: i64) -> String {
    let d = (now - then).max(0);
    match d {
        0..=59 => "just now".into(),
        60..=3599 => format!("{} min ago", d / 60),
        3600..=86399 => format!("{} h ago", d / 3600),
        86400..=172799 => "yesterday".into(),
        _ => format!("{} days ago", d / 86400),
    }
}

/// The address bar's text for a page's URL (the start page shows empty).
pub fn url_to_address(url: &str) -> String {
    if url.starts_with("data:") {
        String::new()
    } else {
        stonedgem_url::display(url, true)
    }
}

/// Which tab is active after closing `closed` (of `count` before closing)
/// while `active` was active. `None` when no tab is left.
pub fn after_close(count: usize, active: usize, closed: usize) -> Option<usize> {
    if count <= 1 {
        return None;
    }
    let left = count - 1;
    Some(if closed < active {
        active - 1
    } else if closed == active {
        active.min(left - 1)
    } else {
        active
    })
}

/// Browser commands bound to keys.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Shortcut {
    NewTab,
    CloseTab,
    FocusAddress,
    Find,
    History,
    Bookmark,
    Bookmarks,
    Settings,
    Reload,
    Back,
    Forward,
    NextTab,
    PreviousTab,
}

/// The browser shortcut for a key press, if any.
pub fn shortcut(key: &IcedKey, m: Modifiers) -> Option<Shortcut> {
    match key.as_ref() {
        IcedKey::Character(c) if m.command() => match c.to_lowercase().as_str() {
            "t" => Some(Shortcut::NewTab),
            "w" => Some(Shortcut::CloseTab),
            "l" => Some(Shortcut::FocusAddress),
            "r" => Some(Shortcut::Reload),
            "f" => Some(Shortcut::Find),
            "h" => Some(Shortcut::History),
            "d" => Some(Shortcut::Bookmark),
            "o" if m.shift() => Some(Shortcut::Bookmarks),
            "," => Some(Shortcut::Settings),
            _ => None,
        },
        IcedKey::Named(Named::F5) => Some(Shortcut::Reload),
        IcedKey::Named(Named::Tab) if m.command() => Some(if m.shift() { Shortcut::PreviousTab } else { Shortcut::NextTab }),
        IcedKey::Named(Named::ArrowLeft) if m.alt() => Some(Shortcut::Back),
        IcedKey::Named(Named::ArrowRight) if m.alt() => Some(Shortcut::Forward),
        _ => None,
    }
}

/// The key to send to the page for a press that isn't a shortcut.
pub fn page_key(key: &IcedKey, text: Option<&str>, m: Modifiers) -> Option<Key> {
    if m.command() || m.alt() {
        return None;
    }
    let named = match key.as_ref() {
        IcedKey::Named(n) => match n {
            Named::Enter => Some(Key::Enter),
            Named::Backspace => Some(Key::Backspace),
            Named::Delete => Some(Key::Delete),
            Named::Tab => Some(Key::Tab),
            Named::Escape => Some(Key::Escape),
            Named::ArrowLeft => Some(Key::Left),
            Named::ArrowRight => Some(Key::Right),
            Named::ArrowUp => Some(Key::Up),
            Named::ArrowDown => Some(Key::Down),
            Named::Home => Some(Key::Home),
            Named::End => Some(Key::End),
            Named::PageUp => Some(Key::PageUp),
            Named::PageDown => Some(Key::PageDown),
            _ => None,
        },
        _ => None,
    };
    named.or_else(|| text.filter(|t| !t.is_empty() && !t.chars().any(char::is_control)).map(|t| Key::Text(t.to_string())))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn address_bar() {
        assert_eq!(address_to_url("example.com", SEARCH), "https://example.com/");
        assert!(address_to_url("servo engine", SEARCH).starts_with("https://duckduckgo.com/?q=servo"));
        assert!(address_to_url("servo engine", "kagi").starts_with("https://kagi.com/search?q=servo"));
        assert_eq!(Internal::from_address("stonedgem://history"), Some(Internal::History));
        assert_eq!(Internal::from_address("about:Settings/"), Some(Internal::Settings));
        assert_eq!(Internal::from_address("example.com"), None);
        assert!(recordable("https://a.b/") && !recordable("data:text/html,x"));
        assert_eq!(ago(100, 100), "just now");
        assert_eq!(ago(0, 7200), "2 h ago");
        assert_eq!(url_to_address(START_PAGE), "");
        assert_eq!(url_to_address("https://example.com/a?b=1"), "https://example.com/a?b=1");
    }

    #[test]
    fn closing_tabs() {
        assert_eq!(after_close(1, 0, 0), None);
        assert_eq!(after_close(3, 2, 2), Some(1)); // last tab closed: move left
        assert_eq!(after_close(3, 1, 1), Some(1)); // middle: the next one slides in
        assert_eq!(after_close(3, 2, 0), Some(1)); // a tab before the active one
        assert_eq!(after_close(3, 0, 2), Some(0)); // a tab after it
    }

    #[test]
    fn keys() {
        let ctrl = Modifiers::CTRL;
        let t = IcedKey::Character("t".into());
        assert_eq!(shortcut(&t, ctrl), Some(Shortcut::NewTab));
        assert_eq!(shortcut(&IcedKey::Character("f".into()), ctrl), Some(Shortcut::Find));
        assert_eq!(shortcut(&IcedKey::Character("O".into()), ctrl | Modifiers::SHIFT), Some(Shortcut::Bookmarks));
        assert_eq!(shortcut(&t, Modifiers::empty()), None);
        assert_eq!(page_key(&t, Some("t"), Modifiers::empty()), Some(Key::Text("t".into())));
        assert_eq!(page_key(&t, Some("t"), ctrl), None);
        assert_eq!(page_key(&IcedKey::Named(Named::Enter), Some("\r"), Modifiers::empty()), Some(Key::Enter));
        assert_eq!(shortcut(&IcedKey::Named(Named::Tab), ctrl | Modifiers::SHIFT), Some(Shortcut::PreviousTab));
        assert_eq!(page_key(&IcedKey::Named(Named::Shift), None, Modifiers::empty()), None);
    }
}
