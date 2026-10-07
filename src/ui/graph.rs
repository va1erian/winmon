//! A metric section: title row, history graph, and a usage bar.

use win32ui::Color;
use win32ui::d2d::{D2dCanvas, PathBuilder, PointF, RectF, Stroke};

use super::{Align, Fonts, palette, text_vcenter};
use crate::model::RingBuffer;

pub struct Metric<'a> {
    pub title: &'static str,
    pub detail: String,
    /// Latest value, `0..=1`.
    pub value: Option<f32>,
    pub history: &'a RingBuffer<f32>,
    pub accent: Color,
}

pub fn draw(canvas: &mut D2dCanvas<'_>, r: RectF, fonts: &Fonts, unit: f32, m: Metric<'_>) {
    let header_h = 26.0 * unit;
    let bar_h = 26.0 * unit;
    let gap = 8.0 * unit;

    let header = RectF::new(r.left, r.top, r.right, r.top + header_h);
    let cy = (header.top + header.bottom) / 2.0;
    text_vcenter(
        canvas,
        &fonts.label,
        m.title,
        header.left,
        header.right,
        cy,
        Align::Left,
        m.accent,
    );
    if !m.detail.is_empty() {
        text_vcenter(
            canvas,
            &fonts.small,
            &m.detail,
            header.left,
            header.right,
            cy,
            Align::Right,
            palette::TEXT_DIM,
        );
    }

    let bar = RectF::new(r.left, r.bottom - bar_h, r.right, r.bottom);
    let plot = RectF::new(r.left, header.bottom + gap / 2.0, r.right, bar.top - gap);
    draw_plot(canvas, plot, unit, m.history, m.accent);
    draw_bar(canvas, bar, fonts, unit, m.value, m.accent);
}

fn draw_plot(
    canvas: &mut D2dCanvas<'_>,
    r: RectF,
    unit: f32,
    history: &RingBuffer<f32>,
    accent: Color,
) {
    if r.height() <= 2.0 || r.width() <= 2.0 {
        return;
    }
    canvas.fill_rounded_rect(r, 6.0 * unit, palette::PANEL);
    let grid = Stroke::solid(1.0 * unit);
    for q in [0.25, 0.5, 0.75] {
        let y = r.bottom - r.height() * q;
        canvas.draw_line(
            PointF::new(r.left, y),
            PointF::new(r.right, y),
            palette::GRID,
            grid,
        );
    }
    if history.len() < 2 {
        return;
    }

    // x spaced by the buffer's capacity, newest at the right edge, so the
    // graph fills in from the right as history accumulates.
    let dx = r.width() / (history.capacity() - 1).max(1) as f32;
    let n = history.len();
    let points: Vec<PointF> = history
        .iter_oldest_first()
        .enumerate()
        .map(|(i, v)| {
            let x = r.right - (n - 1 - i) as f32 * dx;
            let y = r.bottom - v.clamp(0.0, 1.0) * r.height();
            PointF::new(x, y)
        })
        .collect();

    let line = |close: bool| {
        let mut b = PathBuilder::new().ok()?;
        b.move_to(points[0]);
        for &p in &points[1..] {
            b.line_to(p);
        }
        if close {
            b.line_to(PointF::new(points[n - 1].x, r.bottom));
            b.line_to(PointF::new(points[0].x, r.bottom));
            b.close();
        }
        b.build().ok()
    };
    if let Some(area) = line(true) {
        canvas.fill_path(&area, palette::alpha(accent, 64));
    }
    if let Some(stroke) = line(false) {
        canvas.stroke_path(&stroke, accent.into(), Stroke::solid(2.0 * unit));
    }
}

fn draw_bar(
    canvas: &mut D2dCanvas<'_>,
    r: RectF,
    fonts: &Fonts,
    unit: f32,
    value: Option<f32>,
    accent: Color,
) {
    let radius = r.height() / 2.0;
    canvas.fill_rounded_rect(r, radius, palette::TRACK);
    let label = match value {
        Some(v) => {
            let v = v.clamp(0.0, 1.0);
            let w = (r.width() * v).max(r.height());
            canvas.fill_rounded_rect_rgba(
                RectF::new(r.left, r.top, r.left + w, r.bottom),
                radius,
                palette::alpha(accent, 200),
            );
            format!("{:.0}%", v * 100.0)
        }
        None => "—".to_string(),
    };
    let cy = (r.top + r.bottom) / 2.0;
    let pad = 10.0 * unit;
    text_vcenter(
        canvas,
        &fonts.label,
        &label,
        r.left + pad,
        r.right - pad,
        cy,
        Align::Right,
        palette::TEXT,
    );
}
