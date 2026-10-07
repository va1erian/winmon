//! Weather icons drawn from primitives: no image assets, no emoji fonts.

use win32ui::Color;
use win32ui::d2d::{Cap, D2dCanvas, PathBuilder, PointF, RectF, Stroke};

use super::palette;
use crate::weather::icons::IconKind;

/// Draws `kind` centred in the largest square that fits `r`.
pub fn draw(canvas: &mut D2dCanvas<'_>, r: RectF, kind: IconKind, unit: f32) {
    let s = r.width().min(r.height());
    if s <= 1.0 {
        return;
    }
    let x = r.left + (r.width() - s) / 2.0;
    let y = r.top + (r.height() - s) / 2.0;
    // Work in a 0..1 box.
    let p = |u: f32, v: f32| PointF::new(x + u * s, y + v * s);
    let line_w = (s * 0.06).max(1.5 * unit);

    match kind {
        IconKind::Clear => sun(canvas, p(0.5, 0.5), s * 0.2, s, line_w),
        IconKind::PartlyCloudy => {
            sun(canvas, p(0.36, 0.36), s * 0.15, s * 0.75, line_w);
            cloud(canvas, &p, s, 0.12, palette::CLOUD);
        }
        IconKind::Cloudy => {
            cloud(
                canvas,
                &|u, v| p(u - 0.1, v - 0.12),
                s * 0.8,
                0.0,
                palette::CLOUD_DARK,
            );
            cloud(canvas, &p, s, 0.05, palette::CLOUD);
        }
        IconKind::Fog => {
            let st = Stroke::solid(line_w).cap(Cap::Round);
            for (i, v) in [0.32f32, 0.5, 0.68].iter().enumerate() {
                let inset = if i == 1 { 0.12 } else { 0.2 };
                canvas.draw_line(p(inset, *v), p(1.0 - inset, *v), palette::CLOUD, st);
            }
        }
        IconKind::Drizzle | IconKind::Rain => {
            cloud(canvas, &p, s, -0.08, palette::CLOUD);
            let st = Stroke::solid(line_w * 0.8).cap(Cap::Round);
            let len = if kind == IconKind::Rain { 0.16 } else { 0.07 };
            for u in [0.36f32, 0.52, 0.68] {
                canvas.draw_line(p(u, 0.68), p(u - len * 0.4, 0.68 + len), palette::RAIN, st);
            }
        }
        IconKind::Snow => {
            cloud(canvas, &p, s, -0.08, palette::CLOUD);
            for (u, v) in [(0.36f32, 0.74f32), (0.52, 0.82), (0.68, 0.74)] {
                canvas.fill_ellipse(p(u, v), s * 0.04, s * 0.04, palette::SNOW);
            }
        }
        IconKind::Thunder => {
            cloud(canvas, &p, s, -0.08, palette::CLOUD_DARK);
            if let Ok(mut b) = PathBuilder::new() {
                b.move_to(p(0.54, 0.58))
                    .line_to(p(0.42, 0.78))
                    .line_to(p(0.52, 0.78))
                    .line_to(p(0.46, 0.94))
                    .line_to(p(0.64, 0.70))
                    .line_to(p(0.54, 0.70))
                    .line_to(p(0.60, 0.58))
                    .close();
                if let Ok(path) = b.build() {
                    canvas.fill_path(&path, palette::BOLT.into());
                }
            }
        }
    }
}

fn sun(canvas: &mut D2dCanvas<'_>, c: PointF, radius: f32, size: f32, line_w: f32) {
    canvas.fill_ellipse(c, radius, radius, palette::SUN);
    let st = Stroke::solid(line_w).cap(Cap::Round);
    let (r0, r1) = (radius * 1.45, (radius * 1.45 + size * 0.1).min(size * 0.48));
    for i in 0..8 {
        let a = i as f32 * std::f32::consts::FRAC_PI_4;
        let (sn, cs) = a.sin_cos();
        canvas.draw_line(
            PointF::new(c.x + cs * r0, c.y + sn * r0),
            PointF::new(c.x + cs * r1, c.y + sn * r1),
            palette::SUN,
            st,
        );
    }
}

/// A cloud from three circles and a rounded base; `dy` shifts it down.
fn cloud(
    canvas: &mut D2dCanvas<'_>,
    p: &dyn Fn(f32, f32) -> PointF,
    s: f32,
    dy: f32,
    color: Color,
) {
    let base_top = p(0.18, 0.52 + dy);
    let base_bottom = p(0.82, 0.70 + dy);
    canvas.fill_rounded_rect(
        RectF::new(base_top.x, base_top.y, base_bottom.x, base_bottom.y),
        s * 0.09,
        color,
    );
    canvas.fill_ellipse(p(0.36, 0.53 + dy), s * 0.14, s * 0.14, color);
    canvas.fill_ellipse(p(0.56, 0.45 + dy), s * 0.19, s * 0.19, color);
    canvas.fill_ellipse(p(0.72, 0.56 + dy), s * 0.11, s * 0.11, color);
}
