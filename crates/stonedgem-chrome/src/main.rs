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

use stonedgem_chrome::downloads::Downloads;
use stonedgem_chrome::palette::Palette;
use stonedgem_chrome::session::Session;
use stonedgem_chrome::settings::Settings;
use stonedgem_chrome::{
    address_to_url, after_close, element_at_script, find, page_key, recordable, shortcut, url_to_address, zoom_step, Internal,
    start_page, Shortcut, TabInfo,
};
use stonedgem_engine::{Engine, Load, Page};
use stonedgem_library::Library;

/// The glass bar above the page: specular line, toolbar (56), tabs (40), hairline.
pub const CHROME_HEIGHT: f32 = 98.0;
/// The find bar, when open.
pub const FIND_HEIGHT: f32 = 44.0;
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

/// What the right-click menu offers, from what was under the pointer.
pub struct ContextMenu {
    pub at: Point,
    pub link: String,
    pub image: String,
    pub selection: String,
}

/// The product name: "StonedGem", or "OyNIx Next" when run as `oynix-next`
/// (which also keeps its own profile folders).
pub fn product() -> &'static str {
    static NAME: std::sync::OnceLock<&'static str> = std::sync::OnceLock::new();
    NAME.get_or_init(|| {
        let exe = std::env::args().next().unwrap_or_default();
        if std::path::Path::new(&exe).file_name().is_some_and(|n| n == "oynix-next") {
            "OyNIx Next"
        } else {
            "StonedGem"
        }
    })
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
    pub downloads: Downloads,
    pub context: Option<ContextMenu>,
    saved_session: Session,
    /// Installed extensions and the guarded scripts of the enabled ones.
    pub extensions: Vec<stonedgem_extensions::registry::Entry>,
    ext_scripts: Vec<String>,
    pub ext_path: String,
    pub ext_message: String,
    /// Held modifier keys (text fields swallow some Ctrl shortcuts as text).
    modifiers: keyboard::Modifiers,
    /// Enabled extensions' Z#++ scripts.
    pub zs: stonedgem_chrome::zs_hooks::Scripts,
    /// A short message at the bottom of the window, until the tick given.
    pub toast: Option<(String, u64)>,
    /// The Z#++ playground.
    pub zs_editor: iced::widget::text_editor::Content,
    pub zs_output: String,
    pub zs_frame: Option<image::Handle>,
    /// The latest page frame's pixels (what glass popups refract).
    last_frame: Option<(u32, u32, Vec<u8>)>,
    /// The refracting glass behind the open popup, and when to redraw it.
    pub glass_bg: Option<image::Handle>,
    glass_pointer: Option<Point>,
    glass_dirty: bool,
    window_h: f32,
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
    RightClicked,
    CloseContext,
    OpenInNewTab(String),
    Copy(String),
    Download(String),
    DownloadCancel(u64),
    DownloadsClear,
    OpenDownloadsFolder,
    Zoom(i32),
    SetRestore(bool),
    Modifiers(keyboard::Modifiers),
    ZsEdit(iced::widget::text_editor::Action),
    GlassPointer(Point),
    ZsRun,
    DismissToast,
    ExtPath(String),
    ExtInstall,
    ExtToggle(String),
    ExtRemove(String),
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
            downloads: Downloads::default(),
            context: None,
            saved_session: Session::default(),
            extensions: vec![],
            ext_scripts: vec![],
            ext_path: String::new(),
            ext_message: String::new(),
            modifiers: keyboard::Modifiers::empty(),
            zs: Default::default(),
            toast: None,
            zs_editor: iced::widget::text_editor::Content::with_text(stonedgem_chrome::zs_hooks::PLAYGROUND_SAMPLE),
            zs_output: String::new(),
            zs_frame: None,
            last_frame: None,
            glass_bg: None,
            glass_pointer: None,
            glass_dirty: false,
            window_h: 800.0,
        };
        b.reload_extensions();
        match Session::load().filter(|_| b.settings.restore_tabs) {
            Some(s) => {
                for url in &s.tabs {
                    match Internal::from_address(url) {
                        Some(page) => b.open_internal(page),
                        None => b.open_web(url),
                    }
                }
                b.select(s.active);
                b.editing = false;
                b.saved_session = s;
            }
            None => {
                let home = b.home_url();
                b.open_web(&home);
            }
        }
        (b, iced::widget::operation::focus(ADDRESS))
    }

    fn home_url(&self) -> String {
        if self.settings.home.trim().is_empty() {
            start_page(product())
        } else {
            address_to_url(&self.settings.home, &self.settings.search)
        }
    }

    fn open_web(&mut self, url: &str) {
        match self.engine.open(url, &self.ext_scripts, &[]) {
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
        if page == Internal::Zsharp && self.zs_output.is_empty() {
            self.run_playground();
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

    pub fn view_width(&self) -> u32 {
        self.view.0
    }

    pub fn tab(&self) -> Option<&Tab> {
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
            Content::Internal(_) => match self.engine.open(url, &self.ext_scripts, &[]) {
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
            Some(t) if !t.info.title.is_empty() => format!("{} — {}", t.info.title, product()),
            _ => product().into(),
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

    /// Read the extension list and rebuild the page scripts. New pages (and
    /// reloaded ones) get them.
    fn reload_extensions(&mut self) {
        self.extensions = stonedgem_extensions::registry::load();
        self.ext_scripts = stonedgem_extensions::registry::page_scripts(&self.extensions);
        stonedgem_blocker::blocker::set_extension_filters(stonedgem_extensions::registry::filter_files(&self.extensions));
        self.zs = stonedgem_chrome::zs_hooks::Scripts::load(&self.extensions);
    }

    fn notify(&mut self, text: impl Into<String>) {
        self.toast = Some((text.into(), self.ticks + 240));
        self.glass_dirty = true;
    }

    pub fn window_height(&self) -> u32 {
        self.window_h as u32
    }

    /// Render the refracting glass behind the open popup from the latest
    /// page frame (Liquid Glass; see stonedgem_chrome::glass).
    fn refresh_glass(&mut self) {
        self.glass_dirty = false;
        let (Some((x, y, w, h)), Some((fw, fh, px))) = (ui::popup_rect(self), self.last_frame.as_ref()) else {
            self.glass_bg = None;
            return;
        };
        if self.tab().and_then(|t| t.page()).is_none() {
            self.glass_bg = None;
            return;
        }
        let p = self.palette;
        let chrome = CHROME_HEIGHT + if self.find.is_some() { FIND_HEIGHT } else { 0.0 };
        let bd = stonedgem_chrome::glass::Backdrop { rgba: px, w: *fw, h: *fh, outside: [p.surface_container.r, p.surface_container.g, p.surface_container.b] };
        let tint = p.surface_container;
        let g = stonedgem_chrome::glass::Glass {
            radius: if self.toast.is_some() && !self.menu && self.context.is_none() { h / 2.0 } else { 20.0 },
            bend: 14.0,
            frost: 4.0,
            tint: [tint.r, tint.g, tint.b, if p.dark { 0.84 } else { 0.80 }],
            rim: if p.dark { [1.0, 1.0, 1.0, 0.35] } else { [1.0, 1.0, 1.0, 0.9] },
        };
        let rgba = stonedgem_chrome::glass::refract(&bd, x, y - chrome, w as u32, h as u32, &g, self.glass_pointer.map(|q| (q.x, q.y)));
        self.glass_bg = Some(image::Handle::from_rgba(w as u32, h as u32, rgba));
    }

    /// Run the Z#++ playground's program and show its output and canvas.
    fn run_playground(&mut self) {
        let src = self.zs_editor.text();
        let result = stonedgem_zsharp::Program::from_source(&src).and_then(|mut p| {
            let r = p.run_main();
            let out = p.output();
            r.map(|_| (out, p))
        });
        match result {
            Ok((out, p)) => {
                self.zs_output = if out.is_empty() { "(ran; printed nothing)".into() } else { out };
                self.zs_frame = ["canvas", "screen"].iter().find_map(|g| match p.interp.global(g) {
                    Some(stonedgem_zsharp::Value::Object(o)) => o
                        .as_any()
                        .downcast_ref::<stonedgem_zsharp::canvas::Canvas>()
                        .map(|c| image::Handle::from_rgba(c.width(), c.height(), c.rgba())),
                    _ => None,
                });
            }
            Err(e) => {
                self.zs_output = format!("Error: {e}");
                self.zs_frame = None;
            }
        }
    }

    fn session(&self) -> Session {
        let mut tabs = vec![];
        let mut active = 0;
        for (i, t) in self.tabs.iter().enumerate() {
            if Session::keep(&t.info.url) {
                if i == self.active {
                    active = tabs.len();
                }
                tabs.push(t.info.url.clone());
            }
        }
        Session { tabs, active }
    }

    fn save_session(&mut self) {
        let s = self.session();
        if s != self.saved_session {
            s.save();
            self.saved_session = s;
        }
    }

    fn quit(&mut self) -> Task<Message> {
        self.library.flush();
        self.save_session();
        iced::exit()
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
                            visits.push((i, t.info.url.clone(), t.info.title.clone()));
                        }
                    }
                }
                for (i, url, title) in visits {
                    self.library.record_visit(&url, &title);
                    // Extensions' Z#++ `page_load` handlers.
                    if !self.zs.is_empty() {
                        for (_ext, action) in self.zs.page_loaded(&url, &title) {
                            use stonedgem_chrome::zs_hooks::{css_js, Action};
                            let js = match action {
                                Action::RunJs(js) => js,
                                Action::Css(css) => css_js(&css),
                                Action::Notify(t) => {
                                    self.notify(t);
                                    continue;
                                }
                            };
                            if let Some(Tab { content: Content::Web(p), .. }) = self.tabs.get(i) {
                                let _ = p.eval(&self.engine, &js, Duration::from_secs(2));
                            }
                        }
                    }
                }
                if self.ticks.is_multiple_of(300) {
                    self.library.flush();
                }
                if self.toast.as_ref().is_some_and(|(_, until)| self.ticks >= *until) {
                    self.toast = None;
                    self.glass_dirty = true;
                }
                // Glass popups follow the page and the pointer, a few times a second.
                if self.glass_dirty && self.ticks.is_multiple_of(4) && (self.menu || self.context.is_some() || self.toast.is_some() || self.glass_bg.is_some()) {
                    self.refresh_glass();
                }
                if self.ticks.is_multiple_of(120) {
                    self.save_session();
                }
                if let Some(p) = self.page() {
                    if p.has_new_frame() {
                        if let Some(f) = p.frame(&self.engine) {
                            self.last_frame = Some((f.width, f.height, f.rgba.clone()));
                            self.frame = Some(image::Handle::from_rgba(f.width, f.height, f.rgba));
                            self.glass_dirty = true;
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
                    // Focus only: the user may already be typing into it.
                    return Task::batch([iced::widget::operation::focus(id), iced::widget::operation::move_cursor_to_end(id)]);
                }
            }
            Message::Resized(size) => {
                self.window_h = size.height;
                self.view = (size.width.max(1.0) as u32, (size.height - CHROME_HEIGHT).max(1.0) as u32);
                if let Some(p) = self.page() {
                    self.engine.resize(p, self.view.0, self.page_height());
                }
            }
            Message::Address(s) => {
                // Iced's text field takes Ctrl+, as typing a comma: treat it
                // as the Settings shortcut instead.
                if self.modifiers.command() && s.chars().count() == self.address.chars().count() + 1 && s.matches(',').count() > self.address.matches(',').count() {
                    return self.update(Message::Open(Internal::Settings));
                }
                self.address = s;
                self.editing = true;
            }
            Message::Modifiers(m) => self.modifiers = m,
            Message::ZsEdit(a) => {
                self.editing = true;
                self.zs_editor.perform(a);
            }
            Message::ZsRun => self.run_playground(),
            Message::DismissToast => self.toast = None,
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
                self.open_web(&start_page(product()));
                self.editing = true;
                self.focus_next = Some(ADDRESS);
            }
            Message::Select(i) => {
                self.select(i);
                self.editing = false;
            }
            Message::Close(i) => match after_close(self.tabs.len(), self.active, i) {
                None => {
                    // Closing the last tab closes the window; it reopens empty.
                    self.tabs.clear();
                    return self.quit();
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
                self.context = None;
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
                if key == keyboard::Key::Named(keyboard::key::Named::Escape) && (self.menu || self.context.is_some() || self.find.is_some()) {
                    if self.menu || self.context.is_some() {
                        self.menu = false;
                        self.context = None;
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
            Message::ToggleMenu => {
                self.menu = !self.menu;
                self.glass_pointer = None;
                self.refresh_glass();
            }
            Message::GlassPointer(q) => {
                self.glass_pointer = Some(q);
                self.glass_dirty = true;
            }
            Message::CloseMenu => {
                self.menu = false;
                self.refresh_glass();
            }
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
                // Take focus off the address bar now, so keys typed before
                // the find field exists don't land there; the next tick
                // focuses the field itself.
                return iced::widget::operation::focus(FIND);
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
            Message::Quit => return self.quit(),
            Message::RightClicked => {
                self.menu = false;
                let Some(p) = self.page() else { return Task::none() };
                let z = p.zoom().max(0.1);
                let (x, y) = (self.pointer.x / z, self.pointer.y / z);
                let answer = match p.eval(&self.engine, &element_at_script(x, y), Duration::from_secs(2)) {
                    Ok(stonedgem_engine::Value::String(s)) => s,
                    _ => String::new(),
                };
                let mut parts = answer.splitn(3, '\n').map(str::to_string);
                self.context = Some(ContextMenu {
                    at: self.pointer,
                    link: parts.next().unwrap_or_default(),
                    image: parts.next().unwrap_or_default(),
                    selection: parts.next().unwrap_or_default(),
                });
                self.glass_pointer = None;
                self.refresh_glass();
            }
            Message::CloseContext => {
                self.context = None;
                self.refresh_glass();
            }
            Message::OpenInNewTab(url) => {
                self.context = None;
                let back = self.active;
                self.open_web(&url);
                self.select(back);
            }
            Message::Copy(t) => {
                self.context = None;
                return iced::clipboard::write(t);
            }
            Message::Download(url) => {
                self.context = None;
                self.downloads.start(&url, &stonedgem_profile::downloads_dir());
            }
            Message::DownloadCancel(id) => self.downloads.cancel(id),
            Message::DownloadsClear => self.downloads.clear_finished(),
            Message::OpenDownloadsFolder => {
                let _ = std::process::Command::new("xdg-open").arg(stonedgem_profile::downloads_dir()).spawn();
            }
            Message::Zoom(dir) => {
                self.context = None;
                if let Some(p) = self.page() {
                    p.set_zoom(if dir == 0 { 1.0 } else { zoom_step(p.zoom(), dir) });
                }
            }
            Message::ExtPath(p) => self.ext_path = p,
            Message::ExtInstall => {
                let path = std::path::PathBuf::from(self.ext_path.trim());
                let mut list = stonedgem_extensions::registry::load();
                self.ext_message = match stonedgem_extensions::registry::install(&path, &mut list) {
                    Ok(e) => {
                        stonedgem_extensions::registry::save(&list);
                        self.ext_path.clear();
                        format!("Added {}. Reload a page to use it.", e.name)
                    }
                    Err(e) => format!("Couldn't add it: {e}"),
                };
                self.reload_extensions();
            }
            Message::ExtToggle(id) => {
                let mut list = stonedgem_extensions::registry::load();
                if let Some(e) = list.iter_mut().find(|e| e.id == id) {
                    e.enabled = !e.enabled;
                }
                stonedgem_extensions::registry::save(&list);
                self.reload_extensions();
            }
            Message::ExtRemove(id) => {
                let mut list = stonedgem_extensions::registry::load();
                stonedgem_extensions::registry::remove(&id, &mut list);
                stonedgem_extensions::registry::save(&list);
                self.reload_extensions();
            }
            Message::SetRestore(on) => {
                self.settings.restore_tabs = on;
                self.save_settings();
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
            Shortcut::Downloads => return self.update(Message::Open(Internal::Downloads)),
            Shortcut::ZoomIn => return self.update(Message::Zoom(1)),
            Shortcut::ZoomOut => return self.update(Message::Zoom(-1)),
            Shortcut::ZoomReset => return self.update(Message::Zoom(0)),
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
                keyboard::Event::ModifiersChanged(m) => Some(Message::Modifiers(m)),
                _ => None,
            }),
        ])
    }
}

fn main() -> iced::Result {
    if product() == "OyNIx Next" {
        stonedgem_profile::set_app("oynix-next");
    }
    // Extension packages: no QML runtime here, so packages with QML panels
    // are refused with a clear message; no trusted keys yet for .poweroynix.
    stonedgem_extensions::host::set(stonedgem_extensions::Host {
        name: product(),
        trusted_keys: "",
        check_qml: None,
        webchannel_js: None,
    });
    iced::application(Browser::boot, Browser::update, Browser::view)
        .title(Browser::title)
        .subscription(Browser::subscription)
        .theme(Browser::theme)
        .window_size(Size::new(1200.0, 800.0))
        .run()
}
