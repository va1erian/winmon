//! The forecast: current conditions, then one column per day.

use win32ui::d2d::{D2dCanvas, RectF};

use super::layout::columns;
use super::{Align, Fonts, Options, icons, palette, text, text_vcenter};
use crate::model::Model;
use crate::weather::icons::kind;

const DAY_ABBR: [&str; 7] = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];

pub fn draw(
    canvas: &mut D2dCanvas<'_>,
    r: RectF,
    fonts: &Fonts,
    unit: f32,
    m: &Model,
    o: &Options,
) {
    canvas.fill_rounded_rect(r, 8.0 * unit, palette::PANEL);
    let pad = 12.0 * unit;
    let inner = RectF::new(r.left + pad, r.top + pad, r.right - pad, r.bottom - pad);

    let Some(f) = &m.forecast else {
        let msg = match &m.weather_error {
            Some(_) => "Weather unavailable",
            None => "Loading weather…",
        };
        let cy = (inner.top + inner.bottom) / 2.0;
        text_vcenter(
            canvas,
            &fonts.label,
            msg,
            inner.left,
            inner.right,
            cy,
            Align::Center,
            palette::TEXT_FAINT,
        );
        return;
    };

    // Current conditions row.
    let head_h = inner.height() * 0.36;
    let head = RectF::new(inner.left, inner.top, inner.right, inner.top + head_h);
    let icon_size = head_h;
    if let Some(code) = f.current_code {
        icons::draw(
            canvas,
            RectF::new(head.left, head.top, head.left + icon_size, head.bottom),
            kind(code),
            unit,
        );
    }
    let cy = (head.top + head.bottom) / 2.0;
    let now = f
        .current_temp
        .map_or("—".to_string(), |t| format!("{t:.0}{}", o.temp_unit));
    text_vcenter(
        canvas,
        &fonts.big,
        &now,
        head.left + icon_size + pad,
        head.right,
        cy,
        Align::Left,
        palette::TEXT,
    );

    // Age label: only when the data is stale (the last fetch failed).
    if let Some(at) = m.forecast_at
        && m.weather_error.is_some()
    {
        let mins = at.elapsed().as_secs() / 60;
        let age = if mins >= 120 {
            format!("{}h ago", mins / 60)
        } else {
            format!("{mins} min ago")
        };
        text_vcenter(
            canvas,
            &fonts.small,
            &age,
            head.left,
            head.right,
            cy,
            Align::Right,
            palette::TEMP_WARM,
        );
    }

    // Day columns.
    let body = RectF::new(
        inner.left,
        head.bottom + 6.0 * unit,
        inner.right,
        inner.bottom,
    );
    let cols = columns(body, f.days.len(), 6.0 * unit);
    for (col, day) in cols.iter().zip(&f.days) {
        let line = fonts.label.metrics().line_height();
        let name = day
            .weekday
            .and_then(|w| DAY_ABBR.get(w as usize))
            .copied()
            .unwrap_or("");
        text(
            canvas,
            &fonts.label,
            name,
            col.left,
            col.right,
            col.top,
            Align::Center,
            palette::TEXT_DIM,
        );

        let rain_h = fonts.small.metrics().line_height();
        let temps_top = col.bottom - rain_h - line;
        let icon_top = col.top + line;
        let icon_h = (temps_top - icon_top).max(0.0);
        let icon_w = icon_h.min(col.width());
        let cx = (col.left + col.right) / 2.0;
        if let Some(code) = day.code {
            icons::draw(
                canvas,
                RectF::new(
                    cx - icon_w / 2.0,
                    icon_top,
                    cx + icon_w / 2.0,
                    icon_top + icon_h,
                ),
                kind(code),
                unit,
            );
        }

        let fmt = |v: Option<f32>| v.map_or("—".to_string(), |t| format!("{t:.0}°"));
        let hi = fmt(day.max);
        let lo = fmt(day.min);
        let hi_w = fonts.label.width(&hi);
        let lo_w = fonts.small.width(&lo);
        let gap = 4.0 * unit;
        let start = cx - (hi_w + gap + lo_w) / 2.0;
        text(
            canvas,
            &fonts.label,
            &hi,
            start,
            start + hi_w,
            temps_top,
            Align::Left,
            palette::TEXT,
        );
        let small_top = temps_top + (line - rain_h) * 0.8;
        text(
            canvas,
            &fonts.small,
            &lo,
            start + hi_w + gap,
            col.right,
            small_top,
            Align::Left,
            palette::TEXT_DIM,
        );

        if let Some(p) = day.rain_pct {
            let color = if p >= 50.0 {
                palette::RAIN
            } else {
                palette::TEXT_FAINT
            };
            text(
                canvas,
                &fonts.small,
                &format!("{p:.0}%"),
                col.left,
                col.right,
                col.bottom - rain_h,
                Align::Center,
                color,
            );
        }
    }
}
