// Neubrutalism: shape and type, written by Theme Studio.
// Basic Rust: constants only; StonedGem reads it, nothing compiles it.

pub const RADIUS: f32 = 6.0;
pub const BORDER_WIDTH: f32 = 3.0;
pub const SHADOW_X: f32 = 4.0;
pub const SHADOW_Y: f32 = 4.0;
pub const SHADOW_BLUR: f32 = 0.0;
pub const FONT_WEIGHT: f32 = 800.0;
pub const GLASS: bool = false;
pub const FONT_FAMILY: &str = "Archivo Black, Arial Black, Helvetica, sans-serif";

pub mod dark {
    pub const BACKGROUND: Color = rgb(24, 24, 27);
    pub const SURFACE: Color = rgb(39, 39, 42);
    pub const SURFACE_ALT: Color = rgb(63, 63, 70);
    pub const TEXT: Color = rgb(250, 250, 250);
    pub const MUTED: Color = rgb(212, 212, 216);
    pub const BORDER: Color = rgb(250, 250, 250);
    pub const SHADOW: Color = rgb(250, 250, 250);
}
