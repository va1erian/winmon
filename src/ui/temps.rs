//! The temperatures row: one tile per reading.

use win32ui::d2d::{D2dCanvas, PointF, RectF};

use super::layout::columns;
use super::{Align, Fonts, Options, palette, text_vcenter};
use crate::model::Model;
use crate::temps::TempReading;

pub fn draw(
    canvas: &mut D2dCanvas<'_>,
    r: RectF,
    fonts: &Fonts,
    unit: f32,
    m: &Model,
    o: &Options,
) {
    let placeholder;
    let readings: &[TempReading] = if m.temps.is_empty() {
        placeholder = [
            TempReading {
                label: "CPU".into(),
                celsius: None,
            },
            TempReading {
                label: "GPU".into(),
                celsius: None,
            },
        ];
        &placeholder
    } else {
        &m.temps
    };

    let tiles = columns(r, readings.len(), 12.0 * unit);
    for (tile, reading) in tiles.iter().zip(readings) {
        canvas.fill_rounded_rect(*tile, 8.0 * unit, palette::PANEL);
        let pad = 14.0 * unit;
        let cy = (tile.top + tile.bottom) / 2.0;
        text_vcenter(
            canvas,
            &fonts.label,
            &reading.label,
            tile.left + pad,
            tile.right - pad,
            cy,
            Align::Left,
            palette::TEXT_DIM,
        );
        let (value, color) = match reading.celsius {
            Some(c) if m.temps_error.is_none() => {
                (format!("{c:.0}°"), palette::temperature(c, o.warm, o.hot))
            }
            _ => ("—".to_string(), palette::TEXT_FAINT),
        };
        text_vcenter(
            canvas,
            &fonts.value,
            &value,
            tile.left + pad,
            tile.right - pad,
            cy,
            Align::Right,
            color,
        );
    }

    // A small status dot when the source is failing.
    if m.temps_error.is_some() {
        let d = 4.0 * unit;
        canvas.fill_ellipse(
            PointF::new(r.right - d * 2.0, r.top + d * 2.0),
            d,
            d,
            palette::ERROR,
        );
    }
}
