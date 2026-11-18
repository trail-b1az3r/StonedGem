//! Drawing the window, all with Iced. The look follows docs/DESIGN.md:
//! Material 3 colour roles, shape and type scales, and state layers; a
//! Liquid-Glass-style layer split (content at the back, controls on a
//! translucent floating panel over a soft gradient backdrop); neumorphic
//! raised/inset surfaces used sparingly; and Apple's guidance on target
//! sizes, spacing, hierarchy and contrast.

use iced::border::Radius;
use iced::font::Weight;

use iced::widget::{
    button, column, container, image, mouse_area, pick_list, row, scrollable, space, stack, text, text_input, toggler,
};
use iced::{Alignment, Background, Border, Color, Element, Font, Length, Padding, Shadow, Theme, Vector};

use stonedgem_chrome::downloads::State as DlState;
use stonedgem_chrome::palette::Palette;
use stonedgem_chrome::settings::ACCENTS;
use stonedgem_chrome::{ago, recordable, Internal};

use crate::{product, Browser, Content, Message, ADDRESS, CHROME_HEIGHT, FIND, FIND_HEIGHT, HISTORY_SEARCH};

const BOLD: Font = Font { weight: Weight::Semibold, ..Font::DEFAULT };
const HEAVY: Font = Font { weight: Weight::Bold, ..Font::DEFAULT };

// ── Tokens (docs/DESIGN.md) ──────────────────────────────────────────

const TAB_ROW: f32 = 40.0;
const TOOL_ROW: f32 = 56.0;
const TARGET: f32 = 40.0;

fn radius(r: f32) -> Radius {
    Radius { top_left: r, top_right: r, bottom_right: r, bottom_left: r }
}

fn alpha(c: Color, a: f32) -> Color {
    Color { a, ..c }
}

fn mix(a: Color, b: Color, t: f32) -> Color {
    Color { r: a.r + (b.r - a.r) * t, g: a.g + (b.g - a.g) * t, b: a.b + (b.b - a.b) * t, a: 1.0 }
}

/// The thin light edge that makes a surface read as glass or raised.
fn highlight(p: Palette) -> Color {
    if p.dark {
        Color { r: 1.0, g: 1.0, b: 1.0, a: 0.10 }
    } else {
        Color { r: 1.0, g: 1.0, b: 1.0, a: 0.70 }
    }
}

fn shadow(y: f32, blur: f32, a: f32) -> Shadow {
    Shadow { color: alpha(Color::BLACK, a), offset: Vector::new(0.0, y), blur_radius: blur }
}

/// M3 state layer over `base` for hover / press.
fn state(p: Palette, base: Option<Color>, s: button::Status) -> Option<Background> {
    let over = match s {
        button::Status::Hovered => 0.08,
        button::Status::Pressed => 0.12,
        _ => 0.0,
    };
    match (base, over > 0.0) {
        (Some(c), true) => Some(Background::Color(mix(c, p.on_surface, over))),
        (Some(c), false) => Some(Background::Color(c)),
        (None, true) => Some(Background::Color(alpha(p.on_surface, over))),
        (None, false) => None,
    }
}

/// The window backdrop: the plain surface (no gradient).
fn backdrop(p: Palette) -> container::Style {
    container::Style { background: Some(Background::Color(p.surface)), ..Default::default() }
}

// ── Liquid Glass (docs/DESIGN.md) ────────────────────────────────────

/// Translucent glass for controls in use (hovered buttons, tabs).
fn glass_fill(p: Palette) -> Color {
    alpha(p.surface_highest, if p.dark { 0.55 } else { 0.62 })
}

/// Glass at rest that must stay legible (address field, selected tab).
fn glass_strong(p: Palette) -> Color {
    alpha(p.surface_highest, if p.dark { 0.78 } else { 0.86 })
}

/// The light rim around glass.
fn glass_rim(p: Palette) -> Color {
    if p.dark {
        alpha(Color::WHITE, 0.14)
    } else {
        alpha(p.outline_variant, 0.65)
    }
}

/// The bright specular line along the top edge of glass.
fn specular(p: Palette) -> Color {
    if p.dark {
        alpha(Color::WHITE, 0.20)
    } else {
        alpha(Color::WHITE, 0.95)
    }
}

/// The translucent toolbar/tab bar.
fn glass_bar(p: Palette) -> container::Style {
    container::Style { background: Some(Background::Color(alpha(p.surface_container, if p.dark { 0.88 } else { 0.86 }))), ..Default::default() }
}

/// A 1 px line in `c`, full width.
fn line<'a>(c: Color) -> Element<'a, Message> {
    container(space()).width(Length::Fill).height(1).style(move |_: &Theme| container::Style { background: Some(Background::Color(c)), ..Default::default() }).into()
}

/// Cards inside scrolling pages: raised by tone and a light edge only.
/// (No shadow: Iced's CPU renderer doesn't clip shadows to the scroll
/// area, so they would spill over the panel.)
fn card_style(p: Palette) -> container::Style {
    container::Style {
        background: Some(Background::Color(alpha(p.surface_container, if p.dark { 0.82 } else { 0.86 }))),
        border: Border { radius: radius(24.0), width: 1.0, color: highlight(p) },
        ..Default::default()
    }
}

fn popup_style(p: Palette) -> container::Style {
    container::Style {
        background: Some(Background::Color(alpha(p.surface_container, 0.96))),
        border: Border { radius: radius(20.0), width: 1.0, color: highlight(p) },
        shadow: shadow(16.0, 40.0, if p.dark { 0.45 } else { 0.18 }),
        ..Default::default()
    }
}

// ── Building blocks ──────────────────────────────────────────────────

fn hairline<'a>(p: Palette) -> Element<'a, Message> {
    container(space())
        .width(Length::Fill)
        .height(1)
        .style(move |_: &Theme| container::Style { background: Some(Background::Color(alpha(p.outline_variant, 0.6))), ..Default::default() })
        .into()
}

/// A round toolbar button with a ≥ 40 px target.
fn icon<'a>(p: Palette, glyph: &'a str, msg: Option<Message>) -> Element<'a, Message> {
    button(text(glyph).size(18).align_x(Alignment::Center).align_y(Alignment::Center).width(Length::Fill).height(Length::Fill))
        .width(TARGET)
        .height(TARGET)
        .padding(0)
        .on_press_maybe(msg)
        .style(move |_: &Theme, s: button::Status| {
            // Liquid Glass: clear at rest, a glass capsule with a rim under the pointer.
            let text_color = if s == button::Status::Disabled { alpha(p.on_surface_variant, 0.38) } else { p.on_surface_variant };
            let glassy = matches!(s, button::Status::Hovered | button::Status::Pressed);
            let bg = match s {
                button::Status::Hovered => Some(Background::Color(glass_fill(p))),
                button::Status::Pressed => Some(Background::Color(glass_strong(p))),
                _ => None,
            };
            button::Style {
                background: bg,
                text_color,
                border: Border { radius: radius(TARGET / 2.0), width: if glassy { 1.0 } else { 0.0 }, color: glass_rim(p) },
                ..Default::default()
            }
        })
        .into()
}

/// A list/menu row: transparent until hovered; `selected` uses the tonal container.
fn flat(p: Palette, selected: bool) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_: &Theme, s: button::Status| {
        let base = selected.then_some(p.secondary_container);
        let text_color = if s == button::Status::Disabled {
            alpha(p.on_surface, 0.38)
        } else if selected {
            p.on_secondary_container
        } else {
            p.on_surface
        };
        button::Style { background: state(p, base, s), text_color, border: Border { radius: radius(12.0), ..Default::default() }, ..Default::default() }
    }
}

/// M3 filled button (the one primary action on a page).
fn filled(p: Palette) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_: &Theme, s: button::Status| {
        let bg = match s {
            button::Status::Hovered => mix(p.primary, p.on_primary, 0.08),
            button::Status::Pressed => mix(p.primary, p.on_primary, 0.12),
            _ => p.primary,
        };
        button::Style {
            background: Some(Background::Color(bg)),
            text_color: p.on_primary,
            border: Border { radius: radius(20.0), ..Default::default() },
            shadow: if s == button::Status::Hovered { shadow(2.0, 6.0, 0.2) } else { Shadow::default() },
            ..Default::default()
        }
    }
}

/// M3 tonal button (secondary actions).
fn tonal(p: Palette) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_: &Theme, s: button::Status| button::Style {
        background: state(p, Some(p.secondary_container), s),
        text_color: p.on_secondary_container,
        border: Border { radius: radius(20.0), ..Default::default() },
        ..Default::default()
    }
}

/// A text field: softly inset; a 2 px primary ring when focused.
fn field(p: Palette, inset: bool) -> impl Fn(&Theme, text_input::Status) -> text_input::Style {
    move |_: &Theme, s: text_input::Status| {
        let focused = matches!(s, text_input::Status::Focused { .. });
        let (bg, edge) = if inset {
            (alpha(p.surface_lowest, if p.dark { 0.55 } else { 0.65 }), alpha(p.outline_variant, 0.7))
        } else {
            (Color::TRANSPARENT, Color::TRANSPARENT)
        };
        text_input::Style {
            background: Background::Color(bg),
            border: Border { radius: radius(20.0), width: if focused { 2.0 } else if inset { 1.0 } else { 0.0 }, color: if focused { p.primary } else { edge } },
            icon: p.on_surface_variant,
            placeholder: p.on_surface_variant,
            value: p.on_surface,
            selection: alpha(p.primary, 0.35),
        }
    }
}

fn card<'a>(p: Palette, content: impl Into<Element<'a, Message>>) -> Element<'a, Message> {
    container(content).padding(24).width(Length::Fill).style(move |_: &Theme| card_style(p)).into()
}

fn chip<'a>(p: Palette, label: String, msg: Message, live: bool) -> Element<'a, Message> {
    button(text(label).size(12).font(BOLD))
        .on_press(msg)
        .padding(Padding::from([6, 12]))
        .style(move |_: &Theme, s: button::Status| button::Style {
            background: state(p, Some(if live { p.primary } else { p.secondary_container }), s),
            text_color: if live { p.on_primary } else { p.on_secondary_container },
            border: Border { radius: radius(16.0), ..Default::default() },
            ..Default::default()
        })
        .into()
}

// ── Window ───────────────────────────────────────────────────────────

pub fn view(b: &Browser) -> Element<'_, Message> {
    let p = b.palette;
    // The classic layout: a full-width translucent bar holding the toolbar,
    // then the tabs (and the find bar), with a specular line on top and a
    // hairline below.
    let mut bar = column![line(specular(p)), container(toolbar(b)).padding(Padding::from([0, 8])), container(tab_strip(b)).padding(Padding::from([0, 8]))];
    if b.find.is_some() {
        bar = bar.push(container(find_bar(b)).padding(Padding::from([0, 8])));
    }
    let bar = container(column![bar, line(alpha(p.outline_variant, 0.7))]).style(move |_: &Theme| glass_bar(p));

    let main = column![bar, content(b)];
    let base = container(main).width(Length::Fill).height(Length::Fill).style(move |_: &Theme| backdrop(p));

    let chrome = CHROME_HEIGHT + if b.find.is_some() { FIND_HEIGHT } else { 0.0 };
    if b.menu {
        return stack![
            base,
            mouse_area(container(space()).width(Length::Fill).height(Length::Fill)).on_press(Message::CloseMenu),
            container(menu(b)).width(Length::Fill).align_x(Alignment::End).padding(Padding { top: chrome - 10.0, right: 16.0, bottom: 0.0, left: 0.0 }),
        ]
        .into();
    }
    if let Some(c) = &b.context {
        let (left, top, _, _) = popup_rect(b).unwrap_or((8.0, chrome, 0.0, 0.0));
        return stack![
            base,
            mouse_area(container(space()).width(Length::Fill).height(Length::Fill))
                .on_press(Message::CloseContext)
                .on_right_press(Message::CloseContext),
            container(context_menu(b, c)).padding(Padding { top, right: 0.0, bottom: 0.0, left }),
        ]
        .into();
    }
    match &b.toast {
        Some((t, _)) => stack![base, toast(b, t)].into(),
        None => base.into(),
    }
}

/// A glass capsule at the bottom centre with a short message (refracting
/// the page behind it).
fn toast<'a>(b: &'a Browser, t: &str) -> Element<'a, Message> {
    let p = b.palette;
    let w = toast_width(t);
    let label = container(text(t.to_string()).size(14).color(p.on_surface).wrapping(iced::widget::text::Wrapping::None)).width(w).height(TOAST_H).center(Length::Fixed(w));
    let face: Element<'a, Message> = match &b.glass_bg {
        Some(h) => stack![image(h.clone()).width(w).height(TOAST_H).content_fit(iced::ContentFit::Fill), label].into(),
        None => container(label)
            .style(move |_: &Theme| container::Style {
                background: Some(Background::Color(alpha(p.surface_container, 0.96))),
                border: Border { radius: radius(TOAST_H / 2.0), width: 1.0, color: glass_rim(p) },
                ..Default::default()
            })
            .into(),
    };
    let pill = button(face).on_press(Message::DismissToast).padding(0).style(move |_: &Theme, _s| button::Style {
        background: None,
        border: Border { radius: radius(TOAST_H / 2.0), width: 1.0, color: glass_rim(p) },
        shadow: shadow(8.0, 24.0, if p.dark { 0.45 } else { 0.18 }),
        ..Default::default()
    });
    container(mouse_area(pill).on_move(Message::GlassPointer)).width(Length::Fill).height(Length::Fill).align_x(Alignment::Center).align_y(Alignment::End).padding(24).into()
}

/// The Z#++ playground: an editor, Run, the output and the canvas.
fn playground(b: &Browser) -> Element<'_, Message> {
    let p = b.palette;
    let editor = iced::widget::text_editor(&b.zs_editor)
        .on_action(Message::ZsEdit)
        .height(Length::Fill)
        .size(14)
        .font(Font::MONOSPACE)
        .padding(14);
    let head = row![
        column![text("Z#++ playground").size(28).font(HEAVY).color(p.on_surface), text(".zsp code · Canvas 2D · imports need files, so use one program here").size(13).color(p.on_surface_variant)].spacing(2),
        space().width(Length::Fill),
        button(text("Run").size(14)).on_press(Message::ZsRun).padding(Padding::from([10, 24])).style(filled(p)),
    ]
    .align_y(Alignment::Center);
    let preview: Element<'_, Message> = match &b.zs_frame {
        Some(h) => container(image(h.clone()).content_fit(iced::ContentFit::Contain)).width(Length::Fill).height(Length::Fill).center(Length::Fill).into(),
        None => container(text("Draw on a canvas called `canvas` (or `screen`) to see it here.").size(13).color(p.on_surface_variant)).center(Length::Fill).into(),
    };
    let out = scrollable(text(b.zs_output.clone()).size(13).font(Font::MONOSPACE).color(p.on_surface)).height(Length::Fill);
    let right = column![
        container(preview).height(Length::FillPortion(3)).width(Length::Fill).padding(8).style(move |_: &Theme| card_style(p)),
        container(out).height(Length::FillPortion(2)).width(Length::Fill).padding(14).style(move |_: &Theme| card_style(p)),
    ]
    .spacing(12);
    let body = row![container(editor).width(Length::FillPortion(1)).height(Length::Fill).style(move |_: &Theme| card_style(p)), container(right).width(Length::FillPortion(1))]
        .spacing(12)
        .height(Length::Fill);
    container(column![head, body].spacing(16)).padding(20).width(Length::Fill).height(Length::Fill).into()
}

fn tab_strip(b: &Browser) -> Element<'_, Message> {
    let p = b.palette;
    let mut strip = row![].spacing(6).align_y(Alignment::Center);
    for (i, t) in b.tabs.iter().enumerate() {
        let active = i == b.active;
        let dot = text(if t.info.loading { "●" } else { "" }).size(8).color(p.primary);
        let label = text(t.info.label())
            .size(13)
            .font(if active { BOLD } else { Font::DEFAULT })
            .wrapping(iced::widget::text::Wrapping::None)
            .color(if active { p.on_surface } else { p.on_surface_variant });
        let close = button(text("✕").size(10).align_x(Alignment::Center).align_y(Alignment::Center).width(Length::Fill).height(Length::Fill))
            .width(22)
            .height(22)
            .padding(0)
            .on_press(Message::Close(i))
            .style(move |_: &Theme, s: button::Status| button::Style {
                background: state(p, None, s),
                text_color: p.on_surface_variant,
                border: Border { radius: radius(11.0), ..Default::default() },
                ..Default::default()
            });
        let body = button(row![dot, label].spacing(6).align_y(Alignment::Center))
            .on_press(Message::Select(i))
            .padding(Padding::from([6, 2]))
            .width(Length::Fill)
            .style(move |_: &Theme, _s| button::Style { background: None, text_color: p.on_surface, ..Default::default() });
        let pill = container(row![body, close].spacing(2).align_y(Alignment::Center))
            .width(Length::Fixed(196.0))
            .height(32)
            .padding(Padding { top: 0.0, right: 5.0, bottom: 0.0, left: 12.0 })
            .align_y(Alignment::Center);
        // Liquid Glass: the selected tab is a glass pill with a rim.
        let pill = if active {
            pill.style(move |_: &Theme| container::Style {
                background: Some(Background::Color(glass_strong(p))),
                border: Border { radius: radius(16.0), width: 1.0, color: glass_rim(p) },
                ..Default::default()
            })
        } else {
            pill
        };
        strip = strip.push(pill);
    }
    strip = strip.push(icon(p, "+", Some(Message::NewTab)));
    container(strip).width(Length::Fill).height(TAB_ROW).align_y(Alignment::Center).into()
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

    let mut pill = row![].spacing(6).align_y(Alignment::Center).padding(Padding { top: 0.0, right: 2.0, bottom: 0.0, left: 14.0 });
    if url.starts_with("http://") {
        pill = pill.push(text("Not secure").size(12).font(BOLD).color(p.error));
    } else if b.tab().and_then(|t| t.internal()).is_some() {
        pill = pill.push(text(product()).size(12).font(BOLD).color(p.primary));
    }
    pill = pill.push(
        text_input("Search or type an address", &b.address)
            .id(ADDRESS)
            .on_input(Message::Address)
            .on_submit(Message::Go)
            .padding(Padding::from([8, 4]))
            .size(14)
            .style(field(p, false)),
    );
    if web {
        pill = pill.push(icon(p, if starred { "★" } else { "☆" }, Some(Message::ToggleBookmark)));
    }
    // Liquid Glass: the address field is a glass capsule with a rim and a
    // specular line along its top.
    let capsule = container(pill).width(Length::Fill).height(TARGET).align_y(Alignment::Center).style(move |_: &Theme| container::Style {
        background: Some(Background::Color(glass_strong(p))),
        border: Border { radius: radius(TARGET / 2.0), width: 1.0, color: glass_rim(p) },
        ..Default::default()
    });
    let pill = stack![capsule, container(line(specular(p))).padding(Padding { top: 1.0, right: TARGET / 2.0, bottom: 0.0, left: TARGET / 2.0 })];

    let mut right = row![].spacing(6).align_y(Alignment::Center);
    let zoom = page.map(|x| x.zoom()).unwrap_or(1.0);
    if (zoom - 1.0).abs() > 0.01 {
        right = right.push(chip(p, format!("{:.0}%", zoom * 100.0), Message::Zoom(0), false));
    }
    if blocked > 0 {
        right = right.push(chip(p, format!("{blocked} blocked"), Message::Open(Internal::Settings), false));
    }
    let running = b.downloads.running();
    if running > 0 {
        right = right.push(chip(p, format!("⇩ {running}"), Message::Open(Internal::Downloads), true));
    } else if !b.downloads.snapshot().is_empty() {
        right = right.push(icon(p, "⇩", Some(Message::Open(Internal::Downloads))));
    }
    right = right.push(icon(p, "⋮", Some(Message::ToggleMenu)));

    let bar = row![
        icon(p, "←", can_back.then_some(Message::Back)),
        icon(p, "→", can_forward.then_some(Message::Forward)),
        icon(p, "↻", page.is_some().then_some(Message::Reload)),
        icon(p, "⌂", Some(Message::Home)),
        pill,
        right,
    ]
    .spacing(4)
    .align_y(Alignment::Center);
    container(bar).width(Length::Fill).height(TOOL_ROW).align_y(Alignment::Center).into()
}

fn find_bar(b: &Browser) -> Element<'_, Message> {
    let p = b.palette;
    let Some(f) = &b.find else { return space().into() };
    let count = match f.result {
        Some((_, 0)) if !f.query.is_empty() => text("No matches").size(13).color(p.error),
        Some((i, n)) if n > 0 => text(format!("{i} of {n}")).size(13).color(p.on_surface_variant),
        _ => text("").size(13),
    };
    let bar = row![
        text_input("Find in page", &f.query)
            .id(FIND)
            .on_input(Message::FindQuery)
            .on_submit(Message::FindStep(1))
            .padding(Padding::from([7, 16]))
            .size(14)
            .width(320)
            .style(field(p, true)),
        count,
        icon(p, "↑", Some(Message::FindStep(-1))),
        icon(p, "↓", Some(Message::FindStep(1))),
        space().width(Length::Fill),
        icon(p, "✕", Some(Message::FindClose)),
    ]
    .spacing(8)
    .align_y(Alignment::Center);
    column![hairline(p), container(bar).width(Length::Fill).height(FIND_HEIGHT - 1.0).align_y(Alignment::Center)].into()
}

// ── Menus ────────────────────────────────────────────────────────────

// ── Liquid Glass popups ──────────────────────────────────────────────

pub const MENU_W: f32 = 288.0;
pub const CTX_W: f32 = 262.0;

/// Popup height for `rows` 40 px items and `dividers` (9 px), with 2 px
/// spacing and 8 px padding: fixed, so the glass behind lines up exactly.
pub fn popup_height(rows: usize, dividers: usize) -> f32 {
    16.0 + rows as f32 * 40.0 + dividers as f32 * 9.0 + (rows + dividers).saturating_sub(1) as f32 * 2.0
}

fn context_rows(c: &crate::ContextMenu) -> (usize, usize) {
    let (mut r, mut d) = (5, 1);
    if !c.link.is_empty() {
        r += 3;
        d += 1;
    }
    if !c.image.is_empty() {
        r += 3;
        d += 1;
    }
    if !c.selection.trim().is_empty() {
        r += 1;
        d += 1;
    }
    (r, d)
}

/// Extensions with a panel (`panel.slint`) or a page (`extra.html`).
fn ext_entries(b: &Browser) -> impl Iterator<Item = &stonedgem_extensions::registry::Entry> {
    b.extensions.iter().filter(|e| e.enabled && (e.has_slint_panel || e.has_page))
}

/// The main menu's rows and dividers.
pub fn menu_rows(b: &Browser) -> (usize, usize) {
    let n = ext_entries(b).count();
    (9 + n, if n > 0 { 3 } else { 2 })
}

/// Where the open popup sits in the window (x, y, w, h), if one is open.
pub fn popup_rect(b: &Browser) -> Option<(f32, f32, f32, f32)> {
    let chrome = CHROME_HEIGHT + if b.find.is_some() { FIND_HEIGHT } else { 0.0 };
    if b.menu {
        let (r, d) = menu_rows(b);
        return Some((b.window_width() as f32 - 16.0 - MENU_W, chrome - 10.0, MENU_W, popup_height(r, d)));
    }
    if let Some(c) = &b.context {
        let (r, d) = context_rows(c);
        let h = popup_height(r, d);
        let left = c.at.x.min(b.view_width() as f32 - CTX_W - 8.0).max(8.0);
        // Keep it inside the window: open upwards if there isn't room below.
        let top = (chrome + c.at.y).min(b.window_height() as f32 - h - 8.0).max(chrome);
        return Some((left, top, CTX_W, h));
    }
    if let Some((t, _)) = &b.toast {
        let w = toast_width(t);
        return Some(((b.window_width() as f32 - w) / 2.0, b.window_height() as f32 - 24.0 - TOAST_H, w, TOAST_H));
    }
    None
}

const TOAST_H: f32 = 42.0;
fn toast_width(t: &str) -> f32 {
    (t.chars().count() as f32 * 7.6 + 40.0).clamp(120.0, 640.0)
}

/// The popup frame: refracting glass (when the page is behind it) with a
/// light rim, or the plain popup sheet.
fn popup<'a>(b: &'a Browser, content: impl Into<Element<'a, Message>>, width: f32, height: f32) -> Element<'a, Message> {
    let p = b.palette;
    let rim = move |_: &Theme| container::Style {
        border: Border { radius: radius(20.0), width: 1.0, color: glass_rim(p) },
        shadow: shadow(16.0, 40.0, if p.dark { 0.45 } else { 0.18 }),
        ..Default::default()
    };
    let body: Element<'a, Message> = match &b.glass_bg {
        Some(h) => stack![
            image(h.clone()).width(width).height(height).content_fit(iced::ContentFit::Fill),
            container(content).padding(8).width(width).height(height).style(move |_: &Theme| container::Style {
                border: Border { radius: radius(20.0), width: 1.0, color: glass_rim(p) },
                ..Default::default()
            })
        ]
        .into(),
        None => container(content).padding(8).width(width).height(height).style(move |_: &Theme| popup_style(p)).into(),
    };
    mouse_area(container(body).style(rim)).on_move(Message::GlassPointer).into()
}

fn menu_item<'a>(p: Palette, label: &'a str, keys: &'a str, msg: Option<Message>) -> Element<'a, Message> {
    let enabled = msg.is_some();
    button(
        row![
            text(label).size(14).color(if enabled { p.on_surface } else { alpha(p.on_surface, 0.38) }),
            space().width(Length::Fill),
            text(keys).size(12).color(p.on_surface_variant)
        ]
        .align_y(Alignment::Center),
    )
    .on_press_maybe(msg)
    .width(Length::Fill)
    .height(40)
    .padding(Padding::from([0, 14]))
    .style(flat(p, false))
    .into()
}

fn menu_divider<'a>(p: Palette) -> Element<'a, Message> {
    container(hairline(p)).padding(Padding::from([4, 8])).into()
}

fn menu(b: &Browser) -> Element<'_, Message> {
    let p = b.palette;
    let page = b.tab().and_then(|t| t.page());
    let zoom = page.map(|x| x.zoom()).unwrap_or(1.0);
    let zoom_row = row![
        text("Zoom").size(14).color(p.on_surface),
        space().width(Length::Fill),
        icon(p, "−", page.map(|_| Message::Zoom(-1))),
        button(text(format!("{:.0}%", zoom * 100.0)).size(13).align_x(Alignment::Center).width(Length::Fill))
            .width(60)
            .on_press_maybe(page.map(|_| Message::Zoom(0)))
            .style(flat(p, false)),
        icon(p, "+", page.map(|_| Message::Zoom(1))),
    ]
    .align_y(Alignment::Center)
    .height(40)
    .padding(Padding { top: 0.0, right: 0.0, bottom: 0.0, left: 14.0 });
    let list = column![
        menu_item(p, "New tab", "Ctrl+T", Some(Message::NewTab)),
        menu_item(p, "Find in page", "Ctrl+F", page.map(|_| Message::FindOpen)),
        zoom_row,
        menu_divider(p),
        menu_item(p, "History", "Ctrl+H", Some(Message::Open(Internal::History))),
        menu_item(p, "Bookmarks", "Ctrl+Shift+O", Some(Message::Open(Internal::Bookmarks))),
        menu_item(p, "Downloads", "Ctrl+J", Some(Message::Open(Internal::Downloads))),
        menu_item(p, "Settings", "Ctrl+,", Some(Message::Open(Internal::Settings))),
        menu_item(p, "Z#++ playground", "", Some(Message::Open(Internal::Zsharp))),
        menu_divider(p),
    ]
    .spacing(2);
    let mut list = list;
    let mut any = false;
    for e in ext_entries(b) {
        any = true;
        let (hint, msg) = if e.has_slint_panel {
            let open = b.panel.as_ref().is_some_and(|x| x.ext == e.id);
            (if open { "Panel ✓" } else { "Panel" }, if open { Message::ClosePanel } else { Message::OpenPanel(e.id.clone()) })
        } else {
            ("Page", Message::OpenExtPage(e.id.clone()))
        };
        list = list.push(menu_item(p, &e.name, hint, Some(msg)));
    }
    if any {
        list = list.push(menu_divider(p));
    }
    let list = list.push(menu_item(p, "Quit", "", Some(Message::Quit)));
    let (r, d) = menu_rows(b);
    popup(b, list, MENU_W, popup_height(r, d))
}

fn context_menu<'a>(b: &'a Browser, c: &'a crate::ContextMenu) -> Element<'a, Message> {
    let p = b.palette;
    let page = b.tab().and_then(|t| t.page());
    let mut list = column![].spacing(2);
    if !c.link.is_empty() {
        list = list
            .push(menu_item(p, "Open link in new tab", "", Some(Message::OpenInNewTab(c.link.clone()))))
            .push(menu_item(p, "Copy link address", "", Some(Message::Copy(c.link.clone()))))
            .push(menu_item(p, "Save link as download", "", Some(Message::Download(c.link.clone()))))
            .push(menu_divider(p));
    }
    if !c.image.is_empty() {
        list = list
            .push(menu_item(p, "Open image in new tab", "", Some(Message::OpenInNewTab(c.image.clone()))))
            .push(menu_item(p, "Copy image address", "", Some(Message::Copy(c.image.clone()))))
            .push(menu_item(p, "Save image", "", Some(Message::Download(c.image.clone()))))
            .push(menu_divider(p));
    }
    if !c.selection.trim().is_empty() {
        list = list.push(menu_item(p, "Copy", "Ctrl+C", Some(Message::Copy(c.selection.clone())))).push(menu_divider(p));
    }
    list = list
        .push(menu_item(p, "Back", "Alt+←", page.filter(|x| x.can_go_back()).map(|_| Message::Back)))
        .push(menu_item(p, "Forward", "Alt+→", page.filter(|x| x.can_go_forward()).map(|_| Message::Forward)))
        .push(menu_item(p, "Reload", "Ctrl+R", Some(Message::Reload)))
        .push(menu_divider(p))
        .push(menu_item(p, "Bookmark this page", "Ctrl+D", Some(Message::ToggleBookmark)))
        .push(menu_item(p, "Find in page", "Ctrl+F", Some(Message::FindOpen)));
    let (r, d) = context_rows(c);
    popup(b, list, CTX_W, popup_height(r, d))
}

// ── Content ──────────────────────────────────────────────────────────

fn content(b: &Browser) -> Element<'_, Message> {
    match &b.panel {
        Some(panel) => row![page_area(b), side_panel(b, panel)].into(),
        None => page_area(b),
    }
}

/// An extension's Slint panel beside the page: a glass column with the
/// extension's name and a close button above what the panel draws.
fn side_panel<'a>(b: &'a Browser, panel: &'a stonedgem_chrome::panels::Panel) -> Element<'a, Message> {
    let p = b.palette;
    let head = row![
        text(panel.title.as_str()).size(14).font(BOLD).color(p.on_surface),
        space().width(Length::Fill),
        icon(p, "✕", Some(Message::ClosePanel)),
    ]
    .align_y(Alignment::Center)
    .height(crate::PANEL_HEAD)
    .padding(Padding { top: 0.0, right: 4.0, bottom: 0.0, left: 16.0 });
    let body: Element<'a, Message> = match &b.panel_frame {
        Some(h) => image(h.clone()).width(Length::Fill).height(Length::Fill).content_fit(iced::ContentFit::None).into(),
        None => space().width(Length::Fill).height(Length::Fill).into(),
    };
    let body = mouse_area(container(body).width(Length::Fill).height(Length::Fill).align_top(Length::Fill))
        .on_move(Message::PanelMoved)
        .on_press(Message::PanelPressed)
        .on_release(Message::PanelReleased)
        .on_scroll(Message::PanelScrolled);
    row![
        container(space()).width(1).height(Length::Fill).style(move |_: &Theme| container::Style { background: Some(Background::Color(alpha(p.outline_variant, 0.7))), ..Default::default() }),
        container(column![head, line(alpha(p.outline_variant, 0.5)), body]).width(crate::PANEL_W - 1.0).height(Length::Fill).style(move |_: &Theme| glass_bar(p)),
    ]
    .into()
}

fn page_area(b: &Browser) -> Element<'_, Message> {
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
                .on_right_press(Message::RightClicked)
                .on_scroll(Message::Scrolled)
                .into()
        }
        Content::Internal(page) => {
            let body = match page {
                Internal::History => history(b),
                Internal::Bookmarks => bookmarks(b),
                Internal::Downloads => downloads(b),
                Internal::Settings => settings(b),
                Internal::Zsharp => return playground(b),
            };
            scrollable(container(container(body).max_width(760)).width(Length::Fill).align_x(Alignment::Center).padding(Padding::from([24, 24])))
                .width(Length::Fill)
                .height(Length::Fill)
                .into()
        }
    }
}

fn heading<'a>(p: Palette, title: &'a str, subtitle: String) -> Element<'a, Message> {
    column![text(title).size(34).font(HEAVY).color(p.on_surface), text(subtitle).size(14).color(p.on_surface_variant)].spacing(4).into()
}

fn page_header<'a>(p: Palette, title: &'a str, subtitle: String, actions: Element<'a, Message>) -> Element<'a, Message> {
    row![heading(p, title, subtitle), space().width(Length::Fill), actions].align_y(Alignment::End).into()
}

/// A row in a list card: a 44 px+ target that opens `url`, with trailing items.
fn link_row<'a>(p: Palette, title: &str, url: &str, side: Element<'a, Message>, remove: Message) -> Element<'a, Message> {
    let title = if title.trim().is_empty() { url.to_string() } else { title.to_string() };
    let main = button(
        column![
            text(title).size(15).color(p.on_surface).wrapping(iced::widget::text::Wrapping::None),
            text(stonedgem_url::display(url, true)).size(12).color(p.on_surface_variant).wrapping(iced::widget::text::Wrapping::None)
        ]
        .spacing(3),
    )
    .on_press(Message::OpenUrl(url.to_string()))
    .width(Length::Fill)
    .padding(Padding::from([10, 14]))
    .style(flat(p, false));
    row![main, side, icon(p, "✕", Some(remove))].spacing(8).align_y(Alignment::Center).into()
}

fn empty<'a>(p: Palette, line: &'a str) -> Element<'a, Message> {
    card(p, container(text(line).size(15).color(p.on_surface_variant)).width(Length::Fill).align_x(Alignment::Center).padding(24))
}

fn history(b: &Browser) -> Element<'_, Message> {
    let p = b.palette;
    let now = stonedgem_library::now();
    let entries = b.library.search_history(&b.history_query, 400);
    let total = b.library.history.len();
    let mut col = column![
        page_header(
            p,
            "History",
            format!("{total} page{} visited", if total == 1 { "" } else { "s" }),
            button(text("Clear history").size(14)).on_press(Message::HistoryClear).padding(Padding::from([10, 20])).style(tonal(p)).into()
        ),
        text_input("Search history", &b.history_query).id(HISTORY_SEARCH).on_input(Message::HistoryQuery).padding(Padding::from([11, 20])).size(15).style(field(p, true)),
    ]
    .spacing(20);
    if entries.is_empty() {
        col = col.push(empty(p, if b.history_query.is_empty() { "Pages you visit will show up here." } else { "Nothing matches." }));
    } else {
        let mut list = column![].spacing(2);
        for h in &entries {
            list = list.push(link_row(p, &h.title, &h.url, text(ago(h.last_visit, now)).size(12).color(p.on_surface_variant).into(), Message::HistoryRemove(h.url.clone())));
        }
        col = col.push(container(list).padding(8).width(Length::Fill).style(move |_: &Theme| card_style(p)));
    }
    col.into()
}

fn bookmarks(b: &Browser) -> Element<'_, Message> {
    let p = b.palette;
    let mut folders: Vec<&str> = b.library.bookmarks.iter().map(|x| x.folder.as_str()).collect();
    folders.sort();
    folders.dedup();
    let n = b.library.bookmarks.len();
    let mut col = column![heading(p, "Bookmarks", format!("{n} saved"))].spacing(20);
    if b.library.bookmarks.is_empty() {
        col = col.push(empty(p, "Press ☆ in the address bar (or Ctrl+D) to bookmark a page."));
    }
    for folder in folders {
        let name = match folder {
            "" => "Other bookmarks",
            "favorites" => "Favourites",
            f => f,
        };
        let mut list = column![container(text(name).size(13).font(BOLD).color(p.primary)).padding(Padding::from([8, 14]))].spacing(2);
        for bm in b.library.bookmarks.iter().filter(|x| x.folder == folder) {
            list = list.push(link_row(p, &bm.title, &bm.url, space().width(0).into(), Message::BookmarkRemove(bm.url.clone())));
        }
        col = col.push(container(list).padding(8).width(Length::Fill).style(move |_: &Theme| card_style(p)));
    }
    col.into()
}

fn downloads(b: &Browser) -> Element<'_, Message> {
    let p = b.palette;
    let list = b.downloads.snapshot();
    let running = b.downloads.running();
    let actions = row![
        button(text("Open folder").size(14)).on_press(Message::OpenDownloadsFolder).padding(Padding::from([10, 20])).style(tonal(p)),
        button(text("Clear finished").size(14)).on_press(Message::DownloadsClear).padding(Padding::from([10, 20])).style(tonal(p)),
    ]
    .spacing(8);
    let sub = if running > 0 { format!("{running} in progress") } else { format!("Saved to {}", stonedgem_profile::downloads_dir().display()) };
    let mut col = column![page_header(p, "Downloads", sub, actions.into())].spacing(20);
    if list.is_empty() {
        col = col.push(empty(p, "Right-click a link or an image and choose Save to download it."));
        return col.into();
    }
    let mut rows = column![].spacing(4);
    for d in list.iter().rev() {
        let name = d.file.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| stonedgem_chrome::downloads::file_name(&d.url, None));
        let (status, color) = match &d.state {
            DlState::Running => (format!("Downloading · {}", d.progress()), p.primary),
            DlState::Done => (format!("Done · {}", d.progress()), p.on_surface_variant),
            DlState::Failed(e) => (format!("Failed — {e}"), p.error),
            DlState::Cancelled => ("Cancelled".into(), p.on_surface_variant),
        };
        let action: Element<'_, Message> = if d.state == DlState::Running { icon(p, "✕", Some(Message::DownloadCancel(d.id))) } else { space().width(0).into() };
        rows = rows.push(
            container(
                row![
                    column![
                        text(name).size(15).color(p.on_surface),
                        text(stonedgem_url::display(&d.url, true)).size(12).color(p.on_surface_variant).wrapping(iced::widget::text::Wrapping::None),
                        text(status).size(12).font(BOLD).color(color)
                    ]
                    .spacing(3)
                    .width(Length::Fill),
                    action
                ]
                .align_y(Alignment::Center),
            )
            .padding(Padding::from([10, 14])),
        );
    }
    col.push(container(rows).padding(8).width(Length::Fill).style(move |_: &Theme| card_style(p))).into()
}

fn section<'a>(p: Palette, title: &'a str, body: impl Into<Element<'a, Message>>) -> Element<'a, Message> {
    card(p, column![text(title).size(20).font(BOLD).color(p.on_surface), body.into()].spacing(18))
}

/// A settings row: label on the left, control on the right, 44 px tall.
fn setting<'a>(p: Palette, label: &'a str, control: impl Into<Element<'a, Message>>) -> Element<'a, Message> {
    row![text(label).size(15).color(p.on_surface).width(Length::Fill), control.into()].align_y(Alignment::Center).height(44).into()
}

fn switch<'a>(on: bool, f: fn(bool) -> Message) -> Element<'a, Message> {
    toggler(on).on_toggle(f).size(24).into()
}

fn settings(b: &Browser) -> Element<'_, Message> {
    let p = b.palette;
    let s = &b.settings;

    let mut swatches = row![].spacing(10).align_y(Alignment::Center);
    for (_, hex) in ACCENTS {
        let c = Palette::new(hex, s.dark).primary;
        let chosen = s.accent.eq_ignore_ascii_case(hex);
        swatches = swatches.push(
            button(text(if chosen { "✓" } else { "" }).size(14).align_x(Alignment::Center).align_y(Alignment::Center).width(Length::Fill).height(Length::Fill))
                .width(34)
                .height(34)
                .padding(0)
                .on_press(Message::SetAccent(hex.to_string()))
                .style(move |_: &Theme, st: button::Status| button::Style {
                    background: Some(Background::Color(if st == button::Status::Hovered { mix(c, Color::WHITE, 0.12) } else { c })),
                    text_color: Palette::new(hex, p.dark).on_primary,
                    border: Border { radius: radius(17.0), width: if chosen { 2.0 } else { 1.0 }, color: if chosen { p.on_surface } else { highlight(p) } },
                    ..Default::default()
                }),
        );
    }
    let appearance = column![setting(p, "Dark mode", switch(s.dark, Message::SetDark)), setting(p, "Accent colour", swatches)].spacing(6);

    let names: Vec<String> = stonedgem_url::ENGINES.iter().map(|e| e.name.to_string()).collect();
    let current = stonedgem_url::engine(&s.search).name.to_string();
    let engine = pick_list(names, Some(current), |name: String| {
        let id = stonedgem_url::ENGINES.iter().find(|e| e.name == name).map(|e| e.id).unwrap_or(stonedgem_chrome::SEARCH);
        Message::SetSearch(id.to_string())
    })
    .text_size(14)
    .padding(Padding::from([8, 14]));
    let home = row![
        text_input("The start page", &b.home_draft).on_input(Message::HomeDraft).on_submit(Message::SaveHome).padding(Padding::from([9, 16])).size(14).width(300).style(field(p, true)),
        button(text("Save").size(14)).on_press(Message::SaveHome).padding(Padding::from([9, 20])).style(filled(p)),
    ]
    .spacing(8)
    .align_y(Alignment::Center);
    let search = column![setting(p, "Search engine", engine), setting(p, "Home page", home), setting(p, "Reopen tabs on start", switch(s.restore_tabs, Message::SetRestore))].spacing(6);

    let (total, ads, trackers) = stonedgem_blocker::blocker::totals();
    let privacy = column![
        setting(p, "Block ads and trackers", switch(s.blocker, Message::SetBlocker)),
        setting(p, "Hide ad spaces on pages", switch(s.hide_elements, Message::SetHideElements)),
        text(format!("Blocked so far: {total} requests ({ads} ads, {trackers} trackers).")).size(13).color(p.on_surface_variant),
    ]
    .spacing(6);

    let mut ext = column![].spacing(6);
    if b.extensions.is_empty() {
        ext = ext.push(text("No extensions yet. Add an unpacked .oynix folder below.").size(14).color(p.on_surface_variant));
    }
    for e in &b.extensions {
        let mut note = if e.kind == "oynix" { format!("{} · {}", e.version, if e.matches.is_empty() { "no pages".to_string() } else { e.matches.join(", ") }) } else { format!("{} · needs WebExtensions (not in this preview)", e.version) };
        if e.has_script {
            note.push_str(" · Z#++ script");
        }
        ext = ext.push(
            row![
                column![text(e.name.clone()).size(15).color(p.on_surface), text(note).size(12).color(p.on_surface_variant).wrapping(iced::widget::text::Wrapping::None)].spacing(2).width(Length::Fill),
                toggler(e.enabled).on_toggle({
                    let id = e.id.clone();
                    move |_| Message::ExtToggle(id.clone())
                }).size(24),
                icon(p, "✕", Some(Message::ExtRemove(e.id.clone()))),
            ]
            .spacing(10)
            .align_y(Alignment::Center)
            .height(52),
        );
    }
    ext = ext.push(
        row![
            text_input("Folder of an unpacked extension", &b.ext_path).on_input(Message::ExtPath).on_submit(Message::ExtInstall).padding(Padding::from([9, 16])).size(14).style(field(p, true)),
            button(text("Add").size(14)).on_press(Message::ExtInstall).padding(Padding::from([9, 20])).style(filled(p)),
        ]
        .spacing(8)
        .align_y(Alignment::Center),
    );
    if !b.ext_message.is_empty() {
        ext = ext.push(text(b.ext_message.clone()).size(13).color(p.on_surface_variant));
    }
    for err in &b.zs.errors {
        ext = ext.push(text(format!("Script not loaded: {err}")).size(13).color(p.error));
    }

    column![
        heading(p, "Settings", format!("{} preview", product())),
        section(p, "Appearance", appearance),
        section(p, "Search and start", search),
        section(p, "Privacy", privacy),
        section(p, "Extensions", ext),
        section(
            p,
            "About",
            text(format!("{}: Servo renders pages, Iced draws this window, the StonedGem blocker checks every request. MIT licence.", product()))
                .size(14)
                .color(p.on_surface_variant)
        ),
    ]
    .spacing(20)
    .into()
}

