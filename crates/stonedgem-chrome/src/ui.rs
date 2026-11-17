//! Drawing the window, all with Iced: tab strip, toolbar, menu, find bar,
//! the page, and the History, Bookmarks and Settings pages. Colours come
//! from the Material palette (`Palette`), the same roles OyNIx uses.

use iced::border::Radius;
use iced::font::Weight;
use iced::widget::{
    button, column, container, image, mouse_area, pick_list, row, scrollable, space, stack, text, text_input, toggler,
};
use iced::{Alignment, Background, Border, Color, Element, Font, Length, Padding, Shadow, Theme, Vector};

use stonedgem_chrome::palette::Palette;
use stonedgem_chrome::settings::ACCENTS;
use stonedgem_chrome::{ago, recordable, Internal};

use crate::{Browser, Content, Message, ADDRESS, FIND, FIND_HEIGHT, HISTORY_SEARCH};

const BOLD: Font = Font { weight: Weight::Semibold, ..Font::DEFAULT };

fn radius(r: f32) -> Radius {
    Radius { top_left: r, top_right: r, bottom_right: r, bottom_left: r }
}

fn fill(c: Color) -> container::Style {
    container::Style { background: Some(Background::Color(c)), ..Default::default() }
}

fn divider<'a>(c: Color) -> Element<'a, Message> {
    container(space()).width(Length::Fill).height(1).style(move |_: &Theme| fill(c)).into()
}

/// A round, flat toolbar button.
fn icon<'a>(p: Palette, glyph: &'a str, msg: Option<Message>) -> Element<'a, Message> {
    button(text(glyph).size(17).align_x(Alignment::Center).width(Length::Fill))
        .width(34)
        .height(34)
        .padding(Padding::from([6, 0]))
        .on_press_maybe(msg)
        .style(move |_: &Theme, s: button::Status| {
            let bg = match s {
                button::Status::Hovered => Some(Background::Color(p.surface_highest)),
                button::Status::Pressed => Some(Background::Color(p.secondary_container)),
                _ => None,
            };
            let text_color = if s == button::Status::Disabled { p.outline } else { p.on_surface_variant };
            button::Style { background: bg, text_color, border: Border { radius: radius(17.0), ..Default::default() }, ..Default::default() }
        })
        .into()
}

/// A flat text button that highlights on hover (menus, lists).
fn flat(p: Palette, active: bool) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_: &Theme, s: button::Status| {
        let bg = match (active, s) {
            (true, _) => Some(Background::Color(p.secondary_container)),
            (_, button::Status::Hovered) => Some(Background::Color(p.surface_highest)),
            (_, button::Status::Pressed) => Some(Background::Color(p.secondary_container)),
            _ => None,
        };
        let text_color = if active { p.on_secondary_container } else { p.on_surface };
        button::Style { background: bg, text_color, border: Border { radius: radius(10.0), ..Default::default() }, ..Default::default() }
    }
}

fn filled(p: Palette) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_: &Theme, s: button::Status| {
        let bg = if s == button::Status::Hovered { Color { a: 0.88, ..p.primary } } else { p.primary };
        button::Style { background: Some(Background::Color(bg)), text_color: p.on_primary, border: Border { radius: radius(18.0), ..Default::default() }, ..Default::default() }
    }
}

fn field(p: Palette, bg: Color) -> impl Fn(&Theme, text_input::Status) -> text_input::Style {
    move |_: &Theme, s: text_input::Status| {
        let focused = matches!(s, text_input::Status::Focused { .. });
        text_input::Style {
            background: Background::Color(bg),
            border: Border { radius: radius(18.0), width: if focused { 2.0 } else { 0.0 }, color: p.primary },
            icon: p.on_surface_variant,
            placeholder: p.on_surface_variant,
            value: p.on_surface,
            selection: Color { a: 0.35, ..p.primary },
        }
    }
}

fn card<'a>(p: Palette, content: impl Into<Element<'a, Message>>) -> Element<'a, Message> {
    container(content)
        .padding(20)
        .width(Length::Fill)
        .style(move |_: &Theme| container::Style {
            background: Some(Background::Color(p.surface_container)),
            border: Border { radius: radius(16.0), ..Default::default() },
            ..Default::default()
        })
        .into()
}

pub fn view(b: &Browser) -> Element<'_, Message> {
    let p = b.palette;
    let mut main = column![tab_strip(b), toolbar(b), divider(p.outline_variant)];
    if b.find.is_some() {
        main = main.push(find_bar(b)).push(divider(p.outline_variant));
    }
    main = main.push(content(b));
    let base = container(main).style(move |_: &Theme| fill(p.surface));

    if !b.menu {
        return base.into();
    }
    stack![
        base,
        mouse_area(container(space()).width(Length::Fill).height(Length::Fill)).on_press(Message::CloseMenu),
        container(menu(p)).width(Length::Fill).align_x(Alignment::End).padding(Padding { top: 78.0, right: 10.0, bottom: 0.0, left: 0.0 }),
    ]
    .into()
}

fn tab_strip(b: &Browser) -> Element<'_, Message> {
    let p = b.palette;
    let mut strip = row![].spacing(2).padding(Padding { top: 6.0, right: 8.0, bottom: 0.0, left: 8.0 }).align_y(Alignment::End);
    for (i, t) in b.tabs.iter().enumerate() {
        let active = i == b.active;
        let dot = text(if t.info.loading { "●" } else { "" }).size(9).color(p.primary);
        let label = text(t.info.label())
            .size(13)
            .wrapping(iced::widget::text::Wrapping::None)
            .color(if active { p.on_surface } else { p.on_surface_variant });
        let close = button(text("✕").size(11).align_x(Alignment::Center).width(Length::Fill))
            .width(20)
            .height(20)
            .padding(Padding::from([3, 0]))
            .on_press(Message::Close(i))
            .style(move |_: &Theme, s: button::Status| button::Style {
                background: (s == button::Status::Hovered).then_some(Background::Color(p.surface_highest)),
                text_color: p.on_surface_variant,
                border: Border { radius: radius(10.0), ..Default::default() },
                ..Default::default()
            });
        let body = button(row![dot, label].spacing(6).align_y(Alignment::Center))
            .on_press(Message::Select(i))
            .padding(Padding::from([5, 4]))
            .width(Length::Fill)
            .style(move |_: &Theme, _s| button::Style { background: None, text_color: p.on_surface, ..Default::default() });
        let tab = container(row![body, close].spacing(2).align_y(Alignment::Center))
            .width(200)
            .height(30)
            .padding(Padding { top: 0.0, right: 6.0, bottom: 0.0, left: 8.0 })
            .align_y(Alignment::Center)
            .style(move |_: &Theme| container::Style {
                background: active.then_some(Background::Color(p.surface_high)),
                border: Border { radius: Radius { top_left: 10.0, top_right: 10.0, bottom_right: 0.0, bottom_left: 0.0 }, ..Default::default() },
                ..Default::default()
            });
        strip = strip.push(tab);
    }
    strip = strip.push(container(icon(p, "+", Some(Message::NewTab))).padding(Padding { top: 0.0, right: 0.0, bottom: 2.0, left: 4.0 }));
    container(strip).width(Length::Fill).height(36).style(move |_: &Theme| fill(p.surface_lowest)).into()
}

fn toolbar(b: &Browser) -> Element<'_, Message> {
    let p = b.palette;
    let page = b.tab().and_then(|t| t.page());
    let can_back = page.is_some_and(|x| x.can_go_back());
    let can_forward = page.is_some_and(|x| x.can_go_forward());
    let url = b.tab().map(|t| t.info.url.as_str()).unwrap_or("");
    let web = page.is_some() && recordable(url);
    let starred = web && b.library.is_bookmarked(url);
    let blocked = page.map(|x| x.blocked().len()).unwrap_or(0);

    let mut pill = row![].spacing(4).align_y(Alignment::Center).padding(Padding { top: 0.0, right: 4.0, bottom: 0.0, left: 12.0 });
    if url.starts_with("http://") {
        pill = pill.push(text("Not secure").size(12).color(p.error));
    } else if b.tab().and_then(|t| t.internal()).is_some() {
        pill = pill.push(text("StonedGem").size(12).font(BOLD).color(p.primary));
    }
    pill = pill.push(
        text_input("Search or type an address", &b.address)
            .id(ADDRESS)
            .on_input(Message::Address)
            .on_submit(Message::Go)
            .padding(Padding::from([7, 6]))
            .size(14)
            .style(field(p, Color::TRANSPARENT)),
    );
    if web {
        pill = pill.push(icon(p, if starred { "★" } else { "☆" }, Some(Message::ToggleBookmark)));
    }
    let pill = container(pill).width(Length::Fill).height(36).align_y(Alignment::Center).style(move |_: &Theme| container::Style {
        background: Some(Background::Color(p.surface_highest)),
        border: Border { radius: radius(18.0), ..Default::default() },
        ..Default::default()
    });

    let shield: Element<'_, Message> = if blocked > 0 {
        container(text(format!("{blocked} blocked")).size(12).color(p.on_secondary_container))
            .padding(Padding::from([4, 10]))
            .style(move |_: &Theme| container::Style {
                background: Some(Background::Color(p.secondary_container)),
                border: Border { radius: radius(12.0), ..Default::default() },
                ..Default::default()
            })
            .into()
    } else {
        space().width(0).into()
    };

    let bar = row![
        icon(p, "←", can_back.then_some(Message::Back)),
        icon(p, "→", can_forward.then_some(Message::Forward)),
        icon(p, "↻", page.is_some().then_some(Message::Reload)),
        icon(p, "⌂", Some(Message::Home)),
        pill,
        shield,
        icon(p, "⋮", Some(Message::ToggleMenu)),
    ]
    .spacing(4)
    .padding(Padding::from([6, 8]))
    .align_y(Alignment::Center);
    container(bar).width(Length::Fill).height(47).style(move |_: &Theme| fill(p.surface_high)).into()
}

fn menu<'a>(p: Palette) -> Element<'a, Message> {
    let item = |label: &'a str, keys: &'a str, msg: Message| -> Element<'a, Message> {
        button(row![text(label).size(14), space().width(Length::Fill), text(keys).size(12).color(p.on_surface_variant)].align_y(Alignment::Center))
            .on_press(msg)
            .width(Length::Fill)
            .padding(Padding::from([8, 12]))
            .style(flat(p, false))
            .into()
    };
    let list = column![
        item("New tab", "Ctrl+T", Message::NewTab),
        item("Find in page", "Ctrl+F", Message::FindOpen),
        divider(p.outline_variant),
        item("History", "Ctrl+H", Message::Open(Internal::History)),
        item("Bookmarks", "Ctrl+Shift+O", Message::Open(Internal::Bookmarks)),
        item("Settings", "Ctrl+,", Message::Open(Internal::Settings)),
        divider(p.outline_variant),
        item("Quit", "", Message::Quit),
    ]
    .spacing(2)
    .width(260);
    container(list)
        .padding(6)
        .style(move |_: &Theme| container::Style {
            background: Some(Background::Color(p.surface_container)),
            border: Border { radius: radius(14.0), width: 1.0, color: p.outline_variant },
            shadow: Shadow { color: Color { a: 0.45, ..Color::BLACK }, offset: Vector::new(0.0, 6.0), blur_radius: 18.0 },
            ..Default::default()
        })
        .into()
}

fn find_bar(b: &Browser) -> Element<'_, Message> {
    let p = b.palette;
    let Some(f) = &b.find else { return space().into() };
    let count = match f.result {
        Some((_, 0)) if !f.query.is_empty() => text("No matches").size(12).color(p.error),
        Some((i, n)) if n > 0 => text(format!("{i} of {n}")).size(12).color(p.on_surface_variant),
        _ => text("").size(12),
    };
    let bar = row![
        text_input("Find in page", &f.query)
            .id(FIND)
            .on_input(Message::FindQuery)
            .on_submit(Message::FindStep(1))
            .padding(Padding::from([5, 12]))
            .size(13)
            .width(280)
            .style(field(p, p.surface_highest)),
        count,
        icon(p, "↑", Some(Message::FindStep(-1))),
        icon(p, "↓", Some(Message::FindStep(1))),
        space().width(Length::Fill),
        icon(p, "✕", Some(Message::FindClose)),
    ]
    .spacing(6)
    .padding(Padding::from([2, 10]))
    .align_y(Alignment::Center);
    container(bar).width(Length::Fill).height(FIND_HEIGHT - 1.0).align_y(Alignment::Center).style(move |_: &Theme| fill(p.surface_container)).into()
}

fn content(b: &Browser) -> Element<'_, Message> {
    let Some(t) = b.tab() else { return space().into() };
    match &t.content {
        Content::Web(_) => {
            let page: Element<'_, Message> = match &b.frame {
                Some(h) => image(h.clone()).width(Length::Fill).height(Length::Fill).content_fit(iced::ContentFit::None).into(),
                None => space().width(Length::Fill).height(Length::Fill).into(),
            };
            mouse_area(container(page).width(Length::Fill).height(Length::Fill).align_top(Length::Fill))
                .on_move(Message::PointerMoved)
                .on_press(Message::Clicked)
                .on_scroll(Message::Scrolled)
                .into()
        }
        Content::Internal(page) => {
            let body = match page {
                Internal::History => history(b),
                Internal::Bookmarks => bookmarks(b),
                Internal::Settings => settings(b),
            };
            scrollable(container(container(body).max_width(780)).width(Length::Fill).align_x(Alignment::Center).padding(Padding::from([28, 24])))
                .width(Length::Fill)
                .height(Length::Fill)
                .into()
        }
    }
}

fn heading<'a>(p: Palette, title: &'a str) -> iced::widget::Text<'a> {
    text(title).size(28).font(BOLD).color(p.on_surface)
}

fn link_row<'a>(p: Palette, title: &str, url: &str, side: Element<'a, Message>, remove: Message) -> Element<'a, Message> {
    let title = if title.trim().is_empty() { url.to_string() } else { title.to_string() };
    let main = button(column![text(title).size(14).color(p.on_surface), text(stonedgem_url::display(url, true)).size(12).color(p.on_surface_variant)].spacing(2))
        .on_press(Message::OpenUrl(url.to_string()))
        .width(Length::Fill)
        .padding(Padding::from([8, 12]))
        .style(flat(p, false));
    row![main, side, icon(p, "✕", Some(remove))].spacing(8).align_y(Alignment::Center).into()
}

fn history(b: &Browser) -> Element<'_, Message> {
    let p = b.palette;
    let now = stonedgem_library::now();
    let entries = b.library.search_history(&b.history_query, 400);
    let mut list = column![].spacing(2);
    for h in &entries {
        list = list.push(link_row(p, &h.title, &h.url, text(ago(h.last_visit, now)).size(12).color(p.on_surface_variant).into(), Message::HistoryRemove(h.url.clone())));
    }
    if entries.is_empty() {
        list = list.push(text(if b.history_query.is_empty() { "Pages you visit will show up here." } else { "Nothing matches." }).size(14).color(p.on_surface_variant));
    }
    column![
        row![heading(p, "History"), space().width(Length::Fill), button(text("Clear history").size(13)).on_press(Message::HistoryClear).padding(Padding::from([8, 16])).style(filled(p))]
            .align_y(Alignment::Center),
        text_input("Search history", &b.history_query).id(HISTORY_SEARCH).on_input(Message::HistoryQuery).padding(Padding::from([9, 16])).style(field(p, p.surface_highest)),
        card(p, list),
    ]
    .spacing(18)
    .into()
}

fn bookmarks(b: &Browser) -> Element<'_, Message> {
    let p = b.palette;
    let mut folders: Vec<&str> = b.library.bookmarks.iter().map(|x| x.folder.as_str()).collect();
    folders.sort();
    folders.dedup();
    let mut col = column![heading(p, "Bookmarks")].spacing(18);
    if b.library.bookmarks.is_empty() {
        col = col.push(text("Press ☆ in the address bar (or Ctrl+D) to bookmark a page.").size(14).color(p.on_surface_variant));
    }
    for folder in folders {
        let name = match folder {
            "" => "Other bookmarks",
            "favorites" => "Favourites",
            f => f,
        };
        let mut list = column![text(name).size(13).font(BOLD).color(p.primary)].spacing(2);
        for bm in b.library.bookmarks.iter().filter(|x| x.folder == folder) {
            list = list.push(link_row(p, &bm.title, &bm.url, space().width(0).into(), Message::BookmarkRemove(bm.url.clone())));
        }
        col = col.push(card(p, list));
    }
    col.into()
}

fn section<'a>(p: Palette, title: &'a str, body: impl Into<Element<'a, Message>>) -> Element<'a, Message> {
    card(p, column![text(title).size(16).font(BOLD).color(p.on_surface), body.into()].spacing(14))
}

fn settings(b: &Browser) -> Element<'_, Message> {
    let p = b.palette;
    let s = &b.settings;

    let mut swatches = row![].spacing(10);
    for (name, hex) in ACCENTS {
        let c = stonedgem_chrome::palette::Palette::new(hex, s.dark).primary;
        let chosen = s.accent.eq_ignore_ascii_case(hex);
        swatches = swatches.push(
            button(space().width(0))
                .width(30)
                .height(30)
                .on_press(Message::SetAccent(hex.to_string()))
                .style(move |_: &Theme, _s| button::Style {
                    background: Some(Background::Color(c)),
                    border: Border { radius: radius(15.0), width: if chosen { 3.0 } else { 0.0 }, color: p.on_surface },
                    ..Default::default()
                }),
        );
        let _ = name;
    }
    let appearance = column![
        toggler(s.dark).label("Dark").on_toggle(Message::SetDark).size(20).text_size(14),
        row![text("Accent colour").size(14).width(140), swatches].align_y(Alignment::Center),
    ]
    .spacing(14);

    let names: Vec<String> = stonedgem_url::ENGINES.iter().map(|e| e.name.to_string()).collect();
    let current = stonedgem_url::engine(&s.search).name.to_string();
    let search = row![
        text("Search engine").size(14).width(140),
        pick_list(names, Some(current), |name: String| {
            let id = stonedgem_url::ENGINES.iter().find(|e| e.name == name).map(|e| e.id).unwrap_or(stonedgem_chrome::SEARCH);
            Message::SetSearch(id.to_string())
        })
        .text_size(14)
        .padding(Padding::from([6, 12])),
    ]
    .align_y(Alignment::Center);

    let home = row![
        text("Home page").size(14).width(140),
        text_input("The StonedGem start page", &b.home_draft).on_input(Message::HomeDraft).on_submit(Message::SaveHome).padding(Padding::from([7, 14])).size(14).style(field(p, p.surface_highest)),
        button(text("Save").size(13)).on_press(Message::SaveHome).padding(Padding::from([8, 16])).style(filled(p)),
    ]
    .spacing(10)
    .align_y(Alignment::Center);

    let (total, ads, trackers) = stonedgem_blocker::blocker::totals();
    let privacy = column![
        toggler(s.blocker).label("Block ads and trackers").on_toggle(Message::SetBlocker).size(20).text_size(14),
        toggler(s.hide_elements).label("Hide ad spaces on pages").on_toggle(Message::SetHideElements).size(20).text_size(14),
        text(format!("Blocked so far: {total} requests ({ads} ads, {trackers} trackers).")).size(13).color(p.on_surface_variant),
    ]
    .spacing(12);

    column![
        heading(p, "Settings"),
        section(p, "Appearance", appearance),
        section(p, "Search and home", column![search, home].spacing(14)),
        section(p, "Privacy", privacy),
        section(
            p,
            "About",
            text("StonedGem browser preview: Servo renders pages, Iced draws this window, the StonedGem blocker checks every request. MIT licence.")
                .size(13)
                .color(p.on_surface_variant)
        ),
    ]
    .spacing(18)
    .into()
}
