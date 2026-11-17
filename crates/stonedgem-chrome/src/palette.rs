//! The window's colours: a Material 3 scheme (`stonedgem-theme`, the same
//! one OyNIx uses) from the accent colour, as Iced colours.

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
