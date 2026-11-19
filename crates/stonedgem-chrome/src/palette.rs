//! The window's colours and shape: a Material 3 scheme (`stonedgem-theme`,
//! the same one OyNIx uses) from the accent colour — or a `.hypertheme`'s
//! tokens and panel styles ([`Palette::from_hypertheme`]).

use iced::Color;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Palette {
    pub dark: bool,
    pub primary: Color,
    pub on_primary: Color,
    pub secondary_container: Color,
    pub on_secondary_container: Color,
    pub surface: Color,
    pub surface_lowest: Color,
    pub surface_low: Color,
    pub surface_container: Color,
    pub surface_high: Color,
    pub surface_highest: Color,
    pub on_surface: Color,
    pub on_surface_variant: Color,
    pub outline: Color,
    pub outline_variant: Color,
    pub error: Color,
    pub tertiary: Color,
    // ── Style (a hypertheme can change these) ──
    /// Corner rounding, 1.0 = the default shapes.
    pub round: f32,
    /// Panel border width; 0 = the default hairline rims.
    pub edge: f32,
    pub edge_color: Color,
    /// A hard offset shadow (x, y) under panels, in `shadow_color`.
    pub hard: Option<(f32, f32)>,
    pub shadow_color: Color,
    /// Liquid Glass (translucent, specular) or flat solid panels.
    pub glass: bool,
    /// Bold text weight for labels.
    pub heavy: bool,
    /// Per-panel fills (`None`: the scheme's).
    pub toolbar: Option<Color>,
    pub tab: Option<Color>,
    pub card: Option<Color>,
    pub button: Option<Color>,
    pub button_text: Option<Color>,
    pub field: Option<Color>,
    pub popup: Option<Color>,
}

fn hex(s: &str) -> Color {
    let v = u32::from_str_radix(s.trim_start_matches('#'), 16).unwrap_or(0xff00ff);
    Color::from_rgb8((v >> 16) as u8, (v >> 8) as u8, v as u8)
}

impl Palette {
    pub fn new(accent: &str, dark: bool) -> Self {
        let s = stonedgem_theme::material::scheme(accent, dark);
        let r = |k: &str| hex(s[k].as_str().unwrap_or("#ff00ff"));
        Palette {
            dark,
            primary: r("primary"),
            on_primary: r("onPrimary"),
            secondary_container: r("secondaryContainer"),
            on_secondary_container: r("onSecondaryContainer"),
            surface: r("surface"),
            surface_lowest: r("surfaceContainerLowest"),
            surface_low: r("surfaceContainerLow"),
            surface_container: r("surfaceContainer"),
            surface_high: r("surfaceContainerHigh"),
            surface_highest: r("surfaceContainerHighest"),
            on_surface: r("onSurface"),
            on_surface_variant: r("onSurfaceVariant"),
            outline: r("outline"),
            outline_variant: r("outlineVariant"),
            error: r("error"),
            tertiary: r("tertiary"),
            round: 1.0,
            edge: 0.0,
            edge_color: r("outlineVariant"),
            hard: None,
            shadow_color: Color::BLACK,
            glass: true,
            heavy: false,
            toolbar: None,
            tab: None,
            card: None,
            button: None,
            button_text: None,
            field: None,
            popup: None,
        }
    }

    /// The window's look from a resolved hypertheme: its colours as the
    /// scheme's roles, and its shape, borders, shadows and panels.
    pub fn from_hypertheme(r: &stonedgem_hypertheme::Resolved) -> Self {
        let c = |k: &str, d: Color| r.color(k).map(|h| hex(&h)).unwrap_or(d);
        let mix = |a: Color, b: Color, t: f32| Color { r: a.r + (b.r - a.r) * t, g: a.g + (b.g - a.g) * t, b: a.b + (b.b - a.b) * t, a: 1.0 };
        let bg = c("background", Color::WHITE);
        let surface = c("surface", bg);
        let alt = c("surface_alt", surface);
        let text = c("text", Color::BLACK);
        let accent = c("accent", hex("#2f7cf6"));
        let border = c("border", text);
        let panel = |name: &str| r.panel(name, "fill").and_then(|v| v.as_str().map(hex));
        let radius = r.number("radius").unwrap_or(16.0) as f32;
        let edge = r.number("border_width").unwrap_or(0.0) as f32;
        let (sx, sy) = (r.number("shadow_x").unwrap_or(0.0) as f32, r.number("shadow_y").unwrap_or(0.0) as f32);
        Palette {
            dark: r.dark,
            primary: accent,
            on_primary: c("on_accent", Color::WHITE),
            secondary_container: mix(accent, surface, 0.55),
            on_secondary_container: text,
            surface: bg,
            surface_lowest: mix(alt, bg, 0.5),
            surface_low: surface,
            surface_container: surface,
            surface_high: alt,
            surface_highest: alt,
            on_surface: text,
            on_surface_variant: c("muted", text),
            outline: border,
            outline_variant: border,
            error: c("danger", hex("#d93025")),
            tertiary: accent,
            round: (radius / 16.0).clamp(0.0, 2.0),
            edge: edge.clamp(0.0, 8.0),
            edge_color: border,
            hard: (sx != 0.0 || sy != 0.0).then_some((sx.clamp(-12.0, 12.0), sy.clamp(-12.0, 12.0))),
            shadow_color: c("shadow", Color::BLACK),
            glass: r.flag("glass").unwrap_or(false),
            heavy: r.number("font_weight").unwrap_or(400.0) >= 600.0,
            toolbar: panel("toolbar"),
            tab: panel("tab"),
            card: panel("card"),
            button: panel("button"),
            button_text: r.panel("button", "text").and_then(|v| v.as_str().map(hex)),
            field: panel("field"),
            popup: panel("popup"),
        }
    }

    /// A border width: the theme's, or `default`.
    pub fn w(&self, default: f32) -> f32 {
        if self.edge > 0.0 {
            self.edge
        } else {
            default
        }
    }

    /// The theme's hard shadow (or none).
    pub fn hard_shadow(&self) -> iced::Shadow {
        match self.hard {
            Some((x, y)) => iced::Shadow { color: self.shadow_color, offset: iced::Vector::new(x, y), blur_radius: 0.0 },
            None => iced::Shadow::default(),
        }
    }

    /// The Iced theme these colours make (for the built-in widgets).
    pub fn theme(&self) -> iced::Theme {
        iced::Theme::custom(
            "StonedGem",
            iced::theme::Palette {
                background: self.surface,
                text: self.on_surface,
                primary: self.primary,
                success: self.tertiary,
                warning: Color::from_rgb8(0xe0, 0xa1, 0x00),
                danger: self.error,
            },
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dark_and_light_differ_and_have_contrast() {
        let d = Palette::new("#2f7cf6", true);
        let l = Palette::new("#2f7cf6", false);
        let lum = |c: Color| 0.2126 * c.r + 0.7152 * c.g + 0.0722 * c.b;
        assert!(lum(d.surface) < 0.1 && lum(d.on_surface) > 0.7);
        assert!(lum(l.surface) > 0.9 && lum(l.on_surface) < 0.2);
        assert_ne!(Palette::new("#d6456b", true).primary, d.primary);
    }
}
