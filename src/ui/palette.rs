//! Dark, low-glow colours.

use win32ui::Color;
use win32ui::d2d::Rgba;

pub const BACKGROUND: Color = Color::hex(0x000000);
pub const PANEL: Color = Color::hex(0x0E1014);
pub const GRID: Color = Color::hex(0x23262D);
pub const TRACK: Color = Color::hex(0x1A1D23);
pub const TEXT: Color = Color::hex(0xE6E8EC);
pub const TEXT_DIM: Color = Color::hex(0x8A9099);
pub const TEXT_FAINT: Color = Color::hex(0x555B64);

pub const CPU: Color = Color::hex(0x4FC3F7);
pub const MEM: Color = Color::hex(0xB388FF);
pub const TEMP_OK: Color = Color::hex(0x69F0AE);
pub const TEMP_WARM: Color = Color::hex(0xFFB74D);
pub const TEMP_HOT: Color = Color::hex(0xFF5252);
pub const ERROR: Color = Color::hex(0xFF5252);

pub const SUN: Color = Color::hex(0xFFD54F);
pub const CLOUD: Color = Color::hex(0xB0BEC5);
pub const CLOUD_DARK: Color = Color::hex(0x78909C);
pub const RAIN: Color = Color::hex(0x4FC3F7);
pub const SNOW: Color = Color::hex(0xE3F2FD);
pub const BOLT: Color = Color::hex(0xFFD54F);

pub fn alpha(c: Color, a: u8) -> Rgba {
    Rgba::with_alpha(c.r, c.g, c.b, a)
}

/// Accent below `warm`, blending to orange at `warm` and red at `hot`.
pub fn temperature(celsius: f32, warm: f32, hot: f32) -> Color {
    let lead = 10.0;
    if celsius <= warm - lead {
        TEMP_OK
    } else if celsius <= warm {
        TEMP_OK.lerp(TEMP_WARM, (celsius - (warm - lead)) / lead)
    } else if celsius < hot {
        TEMP_WARM.lerp(TEMP_HOT, (celsius - warm) / (hot - warm).max(1.0))
    } else {
        TEMP_HOT
    }
}
