//! `stonedgem-browser`: a browser window on StonedGem — Iced for the
//! chrome, Servo (through `stonedgem-engine`) for pages, the StonedGem
//! blocker on every request.
//!
//! Pages are drawn by reading Servo's frames back into an Iced image
//! ("readback"); a shared GPU surface can replace that later without
//! changing anything above `stonedgem-engine`.

use std::time::Duration;

use iced::widget::{button, column, container, image, mouse_area, row, space, text, text_input};
use iced::{keyboard, mouse, time, window, Element, Length, Point, Size, Subscription, Task, Theme};

use stonedgem_chrome::{address_to_url, after_close, page_key, shortcut, url_to_address, Shortcut, TabInfo, START_PAGE};
use stonedgem_engine::{Engine, Load, Page};

/// Height of the tab strip plus the toolbar, above the page.
const CHROME_HEIGHT: f32 = 78.0;
const ADDRESS: &str = "address";

struct Tab {
    page: Page,
    info: TabInfo,
}

struct Browser {
    engine: Engine,
    tabs: Vec<Tab>,
    active: usize,
    address: String,
    editing: bool,
    frame: Option<image::Handle>,
    view: (u32, u32),
    pointer: Point,
}

#[derive(Debug, Clone)]
enum Message {
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
    PointerMoved(Point),
    Clicked,
    Scrolled(mouse::ScrollDelta),
    Key(keyboard::Key, Option<String>, keyboard::Modifiers),
}

impl Browser {
    fn boot() -> (Self, Task<Message>) {
        stonedgem_blocker::blocker::init(vec![]);
        let view = (1200, (800.0 - CHROME_HEIGHT) as u32);
        let engine = Engine::headless(view.0, view.1).expect("software rendering needs Mesa's EGL (libegl-mesa0)");
        let mut b = Browser { engine, tabs: vec![], active: 0, address: String::new(), editing: true, frame: None, view, pointer: Point::ORIGIN };
        b.open(START_PAGE);
        (b, iced::widget::operation::focus(ADDRESS))
    }

    fn open(&mut self, url: &str) {
        match self.engine.open(url, &[], &[]) {
            Ok(page) => {
                self.tabs.push(Tab { page, info: TabInfo { url: url.into(), loading: true, ..Default::default() } });
                self.select(self.tabs.len() - 1);
            }
            Err(e) => eprintln!("stonedgem: {e}"),
        }
    }

    fn select(&mut self, i: usize) {
        self.active = i.min(self.tabs.len().saturating_sub(1));
        for (j, t) in self.tabs.iter().enumerate() {
            if j != self.active {
                t.page.hide();
            }
        }
        if let Some(t) = self.tabs.get(self.active) {
            t.page.show();
            self.engine.resize(&t.page, self.view.0, self.view.1);
            self.address = url_to_address(&t.info.url);
            self.frame = None;
        }
    }

    fn page(&self) -> Option<&Page> {
        self.tabs.get(self.active).map(|t| &t.page)
    }

    fn title(&self) -> String {
        match self.tabs.get(self.active) {
            Some(t) if !t.info.title.is_empty() => format!("{} — StonedGem", t.info.title),
            _ => "StonedGem".into(),
        }
    }

    fn update(&mut self, m: Message) -> Task<Message> {
        match m {
            Message::Tick => {
                self.engine.spin();
                for (i, t) in self.tabs.iter_mut().enumerate() {
                    if t.page.take_changed() {
                        t.info.title = t.page.title().unwrap_or_default();
                        t.info.url = t.page.url().unwrap_or_default();
                        t.info.loading = t.page.load() != Load::Complete;
                        if i == self.active && !self.editing {
                            self.address = url_to_address(&t.info.url);
                        }
                    }
                }
                if let Some(p) = self.page() {
                    if p.has_new_frame() {
                        if let Some(f) = p.frame(&self.engine) {
                            self.frame = Some(image::Handle::from_rgba(f.width, f.height, f.rgba));
                        }
                    }
                }
            }
            Message::Resized(size) => {
                self.view = (size.width.max(1.0) as u32, (size.height - CHROME_HEIGHT).max(1.0) as u32);
                if let Some(t) = self.tabs.get(self.active) {
                    self.engine.resize(&t.page, self.view.0, self.view.1);
                }
            }
            Message::Address(s) => {
                self.address = s;
                self.editing = true;
            }
            Message::Go => {
                let url = address_to_url(&self.address);
                if url.is_empty() {
                    return Task::none();
                }
                if let Some(p) = self.page() {
                    if let Err(e) = p.navigate(&url) {
                        eprintln!("stonedgem: {e}");
                    }
                }
                self.editing = false;
            }
            Message::NewTab => {
                self.open(START_PAGE);
                self.editing = true;
                return iced::widget::operation::focus(ADDRESS);
            }
            Message::Select(i) => {
                self.select(i);
                self.editing = false;
            }
            Message::Close(i) => match after_close(self.tabs.len(), self.active, i) {
                None => return iced::exit(),
                Some(next) => {
                    self.tabs.remove(i);
                    self.select(next);
                }
            },
            Message::Back => self.page().into_iter().for_each(Page::back),
            Message::Forward => self.page().into_iter().for_each(Page::forward),
            Message::Reload => self.page().into_iter().for_each(Page::reload),
            Message::PointerMoved(p) => {
                self.pointer = p;
                if let Some(page) = self.page() {
                    page.mouse_move(p.x, p.y);
                }
            }
            Message::Clicked => {
                self.editing = false;
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
                if !self.editing {
                    if let (Some(k), Some(page)) = (page_key(&key, text.as_deref(), mods), self.page()) {
                        page.key(&k);
                    }
                }
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
            Shortcut::NextTab if !self.tabs.is_empty() => self.select((self.active + 1) % self.tabs.len()),
            Shortcut::PreviousTab if !self.tabs.is_empty() => self.select((self.active + self.tabs.len() - 1) % self.tabs.len()),
            _ => {}
        }
        Task::none()
    }

    fn view(&self) -> Element<'_, Message> {
        let mut strip = row![].spacing(4).padding([4, 6]);
        for (i, t) in self.tabs.iter().enumerate() {
            let label = if t.info.loading { format!("◌ {}", t.info.label()) } else { t.info.label() };
            let style = if i == self.active { button::primary } else { button::secondary };
            strip = strip.push(
                row![
                    button(text(label).size(13)).on_press(Message::Select(i)).style(style).padding([3, 8]),
                    button(text("×").size(13)).on_press(Message::Close(i)).style(button::text).padding([3, 6]),
                ]
                .spacing(0),
            );
        }
        strip = strip.push(button(text("+").size(14)).on_press(Message::NewTab).style(button::text).padding([3, 8]));

        let can_back = self.page().is_some_and(Page::can_go_back);
        let can_forward = self.page().is_some_and(Page::can_go_forward);
        let blocked = self.page().map(|p| p.blocked().len()).unwrap_or(0);
        let toolbar = row![
            button(text("‹")).on_press_maybe(can_back.then_some(Message::Back)).style(button::text),
            button(text("›")).on_press_maybe(can_forward.then_some(Message::Forward)).style(button::text),
            button(text("⟳")).on_press(Message::Reload).style(button::text),
            text_input("Search or type an address", &self.address).id(ADDRESS).on_input(Message::Address).on_submit(Message::Go).padding(6),
            text(if blocked > 0 { format!("{blocked} blocked") } else { String::new() }).size(12),
        ]
        .spacing(6)
        .padding([2, 6])
        .align_y(iced::Alignment::Center);

        let page: Element<'_, Message> = match &self.frame {
            Some(h) => image(h.clone()).width(Length::Fill).height(Length::Fill).content_fit(iced::ContentFit::None).into(),
            None => space().width(Length::Fill).height(Length::Fill).into(),
        };
        let page = mouse_area(container(page).width(Length::Fill).height(Length::Fill).align_top(Length::Fill))
            .on_move(Message::PointerMoved)
            .on_press(Message::Clicked)
            .on_scroll(Message::Scrolled);

        column![strip, toolbar, page].into()
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
        .theme(|_: &Browser| Theme::Dark)
        .window_size(Size::new(1200.0, 800.0))
        .run()
}
