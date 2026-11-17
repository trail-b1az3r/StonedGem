//! `stonedgem-browser`: a browser window on StonedGem — Iced for the
//! chrome, Servo (through `stonedgem-engine`) for pages, the StonedGem
//! blocker on every request.
//!
//! Pages are drawn by reading Servo's frames back into an Iced image
//! ("readback"); a shared GPU surface can replace that later without
//! changing anything above `stonedgem-engine`. History, Bookmarks and
//! Settings are drawn by the window itself, with Iced.

mod ui;

use std::time::Duration;

use iced::widget::image;
use iced::{keyboard, mouse, time, window, Element, Point, Size, Subscription, Task, Theme};

use stonedgem_chrome::palette::Palette;
use stonedgem_chrome::settings::Settings;
use stonedgem_chrome::{
    address_to_url, after_close, find, page_key, recordable, shortcut, url_to_address, Internal, Shortcut, TabInfo, START_PAGE,
};
use stonedgem_engine::{Engine, Load, Page};
use stonedgem_library::Library;

/// Tab strip plus toolbar, above the page.
pub const CHROME_HEIGHT: f32 = 84.0;
/// The find bar, when open.
pub const FIND_HEIGHT: f32 = 40.0;
pub const ADDRESS: &str = "address";
pub const FIND: &str = "find";
pub const HISTORY_SEARCH: &str = "history-search";

pub enum Content {
    Web(Page),
    Internal(Internal),
}

pub struct Tab {
    pub content: Content,
    pub info: TabInfo,
    /// The URL last written to history, so one load counts once.
    recorded: String,
}

impl Tab {
    pub fn page(&self) -> Option<&Page> {
        match &self.content {
            Content::Web(p) => Some(p),
            Content::Internal(_) => None,
        }
    }

    pub fn internal(&self) -> Option<Internal> {
        match self.content {
            Content::Internal(i) => Some(i),
            Content::Web(_) => None,
        }
    }
}

pub struct FindBar {
    pub query: String,
    /// Tick at which to search for `query` (searching waits for a pause in
    /// typing, so a word costs one search, not one per letter).
    due: Option<u64>,
    /// (current, total) from the page.
    pub result: Option<(usize, usize)>,
}

pub struct Browser {
    engine: Engine,
    pub tabs: Vec<Tab>,
    pub active: usize,
    pub address: String,
    editing: bool,
    pub frame: Option<image::Handle>,
    view: (u32, u32),
    pointer: Point,
    pub menu: bool,
    pub find: Option<FindBar>,
    pub library: Library,
    pub history_query: String,
    pub settings: Settings,
    pub home_draft: String,
    pub palette: Palette,
    ticks: u64,
    /// A field to focus once it's on screen (it may not exist until the
    /// next view, e.g. the find bar right after Ctrl+F).
    focus_next: Option<&'static str>,
}

#[derive(Debug, Clone)]
pub enum Message {
    Tick,
    Resized(Size),
    Address(String),
    Go,
    NewTab,
    Select(usize),
    Close(usize),
    Back,
    Forward,
    Reload,
    Home,
    PointerMoved(Point),
    Clicked,
    Scrolled(mouse::ScrollDelta),
    Key(keyboard::Key, Option<String>, keyboard::Modifiers),
    ToggleMenu,
    CloseMenu,
    Open(Internal),
    ToggleBookmark,
    OpenUrl(String),
    FindOpen,
    FindQuery(String),
    FindStep(i32),
    FindClose,
    HistoryQuery(String),
    HistoryRemove(String),
    HistoryClear,
    BookmarkRemove(String),
    SetSearch(String),
    SetDark(bool),
    SetAccent(String),
    SetBlocker(bool),
    SetHideElements(bool),
    HomeDraft(String),
    SaveHome,
    Quit,
}

impl Browser {
    fn boot() -> (Self, Task<Message>) {
        let settings = Settings::load();
        stonedgem_blocker::blocker::init(vec![]);
        settings.apply_blocker();
        let view = (1200, (800.0 - CHROME_HEIGHT) as u32);
        let engine = Engine::headless(view.0, view.1).expect("software rendering needs Mesa's EGL (libegl-mesa0)");
        let palette = Palette::new(&settings.accent, settings.dark);
        let mut b = Browser {
            engine,
            tabs: vec![],
            active: 0,
            address: String::new(),
            editing: true,
            frame: None,
            view,
            pointer: Point::ORIGIN,
            menu: false,
            find: None,
            library: Library::load(),
            history_query: String::new(),
            home_draft: settings.home.clone(),
            settings,
            palette,
            ticks: 0,
            focus_next: None,
        };
        let home = b.home_url();
        b.open_web(&home);
        (b, iced::widget::operation::focus(ADDRESS))
    }

    fn home_url(&self) -> String {
        if self.settings.home.trim().is_empty() {
            START_PAGE.to_string()
        } else {
            address_to_url(&self.settings.home, &self.settings.search)
        }
    }

    fn open_web(&mut self, url: &str) {
        match self.engine.open(url, &[], &[]) {
            Ok(page) => {
                self.tabs.push(Tab { content: Content::Web(page), info: TabInfo { url: url.into(), loading: true, ..Default::default() }, recorded: String::new() });
                self.select(self.tabs.len() - 1);
            }
            Err(e) => eprintln!("stonedgem: {e}"),
        }
    }

    /// Switch to the tab showing `page`, or open it.
    fn open_internal(&mut self, page: Internal) {
        if let Some(i) = self.tabs.iter().position(|t| t.internal() == Some(page)) {
            self.select(i);
            return;
        }
        let info = TabInfo { title: page.title().into(), url: page.address().into(), loading: false };
        self.tabs.push(Tab { content: Content::Internal(page), info, recorded: String::new() });
        self.select(self.tabs.len() - 1);
    }

    fn page_height(&self) -> u32 {
        let find = if self.find.is_some() { FIND_HEIGHT } else { 0.0 };
        (self.view.1 as f32 - find).max(1.0) as u32
    }

    fn select(&mut self, i: usize) {
        let i = i.min(self.tabs.len().saturating_sub(1));
        if i != self.active && self.find.is_some() {
            // Find belongs to one page: clear its highlights and close it.
            if let Some(f) = self.find.as_mut() {
                f.query.clear();
            }
            self.run_find(1);
            self.find = None;
        }
        self.active = i;
        for (j, t) in self.tabs.iter().enumerate() {
            if let (true, Some(p)) = (j != self.active, t.page()) {
                p.hide();
            }
        }
        self.frame = None;
        if let Some(t) = self.tabs.get(self.active) {
            self.address = if t.internal().is_some() { t.info.url.clone() } else { url_to_address(&t.info.url) };
            if let Some(p) = t.page() {
                p.show();
                self.engine.resize(p, self.view.0, self.page_height());
            }
        }
    }

    fn tab(&self) -> Option<&Tab> {
        self.tabs.get(self.active)
    }

    fn page(&self) -> Option<&Page> {
        self.tab().and_then(Tab::page)
    }

    /// Load `url` in the active tab, turning an internal page into a web one.
    fn navigate(&mut self, url: &str) {
        let Some(t) = self.tabs.get_mut(self.active) else { return self.open_web(url) };
        match &t.content {
            Content::Web(p) => {
                if let Err(e) = p.navigate(url) {
                    eprintln!("stonedgem: {e}");
                }
            }
            Content::Internal(_) => match self.engine.open(url, &[], &[]) {
                Ok(p) => {
                    t.content = Content::Web(p);
                    t.info = TabInfo { url: url.into(), loading: true, ..Default::default() };
                    self.select(self.active);
                }
                Err(e) => eprintln!("stonedgem: {e}"),
            },
        }
        self.editing = false;
    }

    fn title(&self) -> String {
        match self.tab() {
            Some(t) if !t.info.title.is_empty() => format!("{} — StonedGem", t.info.title),
            _ => "StonedGem".into(),
        }
    }

    fn theme(&self) -> Theme {
        self.palette.theme()
    }

    fn save_settings(&mut self) {
        self.palette = Palette::new(&self.settings.accent, self.settings.dark);
        self.settings.apply_blocker();
        if let Err(e) = self.settings.save() {
            eprintln!("stonedgem: saving settings: {e}");
        }
    }

    fn run_find(&mut self, step: i32) {
        let (Some(f), Some(p)) = (self.find.as_ref(), self.page()) else { return };
        let answer = p.eval(&self.engine, &find::script(&f.query, step), Duration::from_secs(2));
        let result = match answer {
            Ok(stonedgem_engine::Value::String(s)) => find::parse(&s),
            _ => None,
        };
        if let Some(f) = self.find.as_mut() {
            f.result = result;
        }
    }

    fn update(&mut self, m: Message) -> Task<Message> {
        if std::env::var_os("STONEDGEM_DEBUG").is_some() && !matches!(m, Message::Tick | Message::PointerMoved(_)) {
            eprintln!("stonedgem: {m:?}");
        }
        match m {
            Message::Tick => {
                self.engine.spin();
                self.ticks += 1;
                let mut visits = vec![];
                for (i, t) in self.tabs.iter_mut().enumerate() {
                    let Content::Web(p) = &t.content else { continue };
                    if p.take_changed() {
                        t.info.title = p.title().unwrap_or_default();
                        t.info.url = p.url().unwrap_or_default();
                        t.info.loading = p.load() != Load::Complete;
                        if i == self.active && !self.editing {
                            self.address = url_to_address(&t.info.url);
                        }
                        if !t.info.loading && recordable(&t.info.url) && t.recorded != t.info.url {
                            t.recorded = t.info.url.clone();
                            visits.push((t.info.url.clone(), t.info.title.clone()));
                        }
                    }
                }
                for (url, title) in visits {
                    self.library.record_visit(&url, &title);
                }
                if self.ticks.is_multiple_of(300) {
                    self.library.flush();
                }
                if let Some(p) = self.page() {
                    if p.has_new_frame() {
                        if let Some(f) = p.frame(&self.engine) {
                            self.frame = Some(image::Handle::from_rgba(f.width, f.height, f.rgba));
                        }
                    }
                }
                if self.find.as_ref().and_then(|f| f.due).is_some_and(|t| self.ticks >= t) {
                    if let Some(f) = self.find.as_mut() {
                        f.due = None;
                    }
                    self.run_find(1);
                }
                if let Some(id) = self.focus_next.take() {
                    return Task::batch([iced::widget::operation::focus(id), iced::widget::operation::select_all(id)]);
                }
            }
            Message::Resized(size) => {
                self.view = (size.width.max(1.0) as u32, (size.height - CHROME_HEIGHT).max(1.0) as u32);
                if let Some(p) = self.page() {
                    self.engine.resize(p, self.view.0, self.page_height());
                }
            }
            Message::Address(s) => {
                self.address = s;
                self.editing = true;
            }
            Message::Go => {
                if let Some(page) = Internal::from_address(&self.address) {
                    self.open_internal(page);
                    return Task::none();
                }
                let url = address_to_url(&self.address, &self.settings.search);
                if !url.is_empty() {
                    self.navigate(&url);
                }
            }
            Message::NewTab => {
                self.menu = false;
                self.open_web(START_PAGE);
                self.editing = true;
                self.focus_next = Some(ADDRESS);
            }
            Message::Select(i) => {
                self.select(i);
                self.editing = false;
            }
            Message::Close(i) => match after_close(self.tabs.len(), self.active, i) {
                None => {
                    self.library.flush();
                    return iced::exit();
                }
                Some(next) => {
                    self.tabs.remove(i);
                    self.select(next);
                }
            },
            Message::Back => self.page().into_iter().for_each(Page::back),
            Message::Forward => self.page().into_iter().for_each(Page::forward),
            Message::Reload => self.page().into_iter().for_each(Page::reload),
            Message::Home => {
                let home = self.home_url();
                self.navigate(&home);
            }
            Message::PointerMoved(p) => {
                self.pointer = p;
                if let Some(page) = self.page() {
                    page.mouse_move(p.x, p.y);
                }
            }
            Message::Clicked => {
                self.editing = false;
                self.menu = false;
                if let Some(page) = self.page() {
                    page.click(self.pointer.x, self.pointer.y);
                }
            }
            Message::Scrolled(d) => {
                let (dx, dy) = match d {
                    mouse::ScrollDelta::Lines { x, y } => (x * 40.0, y * 40.0),
                    mouse::ScrollDelta::Pixels { x, y } => (x, y),
                };
                if let Some(page) = self.page() {
                    // Iced: positive y = wheel up. The engine: positive = down the page.
                    page.scroll(self.pointer.x, self.pointer.y, -dx as f64, -dy as f64);
                }
            }
            Message::Key(key, text, mods) => {
                if let Some(s) = shortcut(&key, mods) {
                    return self.run(s);
                }
                if key == keyboard::Key::Named(keyboard::key::Named::Escape) && (self.menu || self.find.is_some()) {
                    if self.menu {
                        self.menu = false;
                    } else {
                        return self.update(Message::FindClose);
                    }
                    return Task::none();
                }
                if !self.editing {
                    if let (Some(k), Some(page)) = (page_key(&key, text.as_deref(), mods), self.page()) {
                        page.key(&k);
                    }
                }
            }
            Message::ToggleMenu => self.menu = !self.menu,
            Message::CloseMenu => self.menu = false,
            Message::Open(page) => {
                self.menu = false;
                self.open_internal(page);
                if page == Internal::History {
                    self.focus_next = Some(HISTORY_SEARCH);
                }
            }
            Message::ToggleBookmark => {
                if let Some(t) = self.tab().filter(|t| t.page().is_some() && recordable(&t.info.url)) {
                    let (url, title) = (t.info.url.clone(), t.info.title.clone());
                    if self.library.is_bookmarked(&url) {
                        self.library.remove_bookmark_url(&url);
                    } else {
                        self.library.add_bookmark(&url, &title, "");
                    }
                }
            }
            Message::OpenUrl(url) => self.navigate(&url),
            Message::FindOpen => {
                self.menu = false;
                if self.page().is_none() {
                    return Task::none();
                }
                if self.find.is_none() {
                    self.find = Some(FindBar { query: String::new(), due: None, result: None });
                    if let Some(p) = self.page() {
                        self.engine.resize(p, self.view.0, self.page_height());
                    }
                }
                self.editing = true;
                self.focus_next = Some(FIND);
            }
            Message::FindQuery(q) => {
                let at = self.ticks + 15;
                if let Some(f) = self.find.as_mut() {
                    f.query = q;
                    f.due = Some(at);
                }
            }
            Message::FindStep(step) => {
                // Enter right after typing searches now (that's the first match).
                let pending = self.find.as_mut().and_then(|f| f.due.take()).is_some();
                self.run_find(if pending { 1 } else { step });
            }
            Message::FindClose => {
                if let Some(f) = self.find.as_mut() {
                    f.query.clear();
                }
                self.run_find(1);
                self.find = None;
                self.editing = false;
                if let Some(p) = self.page() {
                    self.engine.resize(p, self.view.0, self.page_height());
                }
            }
            Message::HistoryQuery(q) => self.history_query = q,
            Message::HistoryRemove(url) => {
                self.library.remove_history(&url);
                self.library.flush();
            }
            Message::HistoryClear => {
                self.library.clear_history(0);
                self.library.flush();
            }
            Message::BookmarkRemove(url) => self.library.remove_bookmark_url(&url),
            Message::SetSearch(id) => {
                self.settings.search = id;
                self.save_settings();
            }
            Message::SetDark(d) => {
                self.settings.dark = d;
                self.save_settings();
            }
            Message::SetAccent(a) => {
                self.settings.accent = a;
                self.save_settings();
            }
            Message::SetBlocker(on) => {
                self.settings.blocker = on;
                self.save_settings();
            }
            Message::SetHideElements(on) => {
                self.settings.hide_elements = on;
                self.save_settings();
            }
            Message::HomeDraft(s) => self.home_draft = s,
            Message::SaveHome => {
                self.settings.home = self.home_draft.trim().to_string();
                self.save_settings();
            }
            Message::Quit => {
                self.library.flush();
                return iced::exit();
            }
        }
        Task::none()
    }

    fn run(&mut self, s: Shortcut) -> Task<Message> {
        match s {
            Shortcut::NewTab => return self.update(Message::NewTab),
            Shortcut::CloseTab => return self.update(Message::Close(self.active)),
            Shortcut::FocusAddress => {
                self.editing = true;
                return Task::batch([iced::widget::operation::focus(ADDRESS), iced::widget::operation::select_all(ADDRESS)]);
            }
            Shortcut::Reload => return self.update(Message::Reload),
            Shortcut::Back => return self.update(Message::Back),
            Shortcut::Forward => return self.update(Message::Forward),
            Shortcut::Find => return self.update(Message::FindOpen),
            Shortcut::History => return self.update(Message::Open(Internal::History)),
            Shortcut::Bookmarks => return self.update(Message::Open(Internal::Bookmarks)),
            Shortcut::Settings => return self.update(Message::Open(Internal::Settings)),
            Shortcut::Bookmark => return self.update(Message::ToggleBookmark),
            Shortcut::NextTab if !self.tabs.is_empty() => self.select((self.active + 1) % self.tabs.len()),
            Shortcut::PreviousTab if !self.tabs.is_empty() => self.select((self.active + self.tabs.len() - 1) % self.tabs.len()),
            _ => {}
        }
        Task::none()
    }

    fn view(&self) -> Element<'_, Message> {
        ui::view(self)
    }

    fn subscription(&self) -> Subscription<Message> {
        Subscription::batch([
            time::every(Duration::from_millis(16)).map(|_| Message::Tick),
            window::resize_events().map(|(_, size)| Message::Resized(size)),
            keyboard::listen().filter_map(|e| match e {
                keyboard::Event::KeyPressed { key, text, modifiers, .. } => Some(Message::Key(key, text.map(|t| t.to_string()), modifiers)),
                _ => None,
            }),
        ])
    }
}

fn main() -> iced::Result {
    iced::application(Browser::boot, Browser::update, Browser::view)
        .title(Browser::title)
        .subscription(Browser::subscription)
        .theme(Browser::theme)
        .window_size(Size::new(1200.0, 800.0))
        .run()
}
