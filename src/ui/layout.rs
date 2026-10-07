//! Pure layout arithmetic: window bounds → section rects (DIPs).

use win32ui::d2d::RectF;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Sections {
    pub clock: RectF,
    pub cpu: RectF,
    pub mem: RectF,
    pub temps: RectF,
    pub weather: RectF,
}

impl Sections {
    pub fn all(&self) -> [RectF; 5] {
        [self.clock, self.cpu, self.mem, self.temps, self.weather]
    }
}

/// The scale factor every size is multiplied by: 1.0 on the reference
/// 480×800 portrait panel, whatever the orientation.
pub fn unit(bounds: RectF) -> f32 {
    (bounds.width().min(bounds.height()) / 480.0).max(0.1)
}

/// Portrait: clock, CPU, memory, temps, weather stacked top to bottom.
/// Landscape (`width > height`): clock, temps and weather on the left, the
/// two graphs on the right.
pub fn sections(bounds: RectF) -> Sections {
    let pad = 12.0 * unit(bounds);
    let inner = inset(bounds, pad);
    if bounds.width() > bounds.height() {
        let [left, right] = split_h(inner, &[0.5, 0.5], pad);
        let [clock, temps, weather] = split_v(left, &[0.30, 0.18, 0.52], pad);
        let [cpu, mem] = split_v(right, &[0.5, 0.5], pad);
        Sections {
            clock,
            cpu,
            mem,
            temps,
            weather,
        }
    } else {
        let [clock, cpu, mem, temps, weather] =
            split_v(inner, &[0.15, 0.22, 0.22, 0.11, 0.30], pad);
        Sections {
            clock,
            cpu,
            mem,
            temps,
            weather,
        }
    }
}

pub fn inset(r: RectF, by: f32) -> RectF {
    let dx = by.min(r.width() / 2.0);
    let dy = by.min(r.height() / 2.0);
    RectF::new(r.left + dx, r.top + dy, r.right - dx, r.bottom - dy)
}

/// Splits `r` top to bottom by `weights`, with `gap` between parts.
pub fn split_v<const N: usize>(r: RectF, weights: &[f32; N], gap: f32) -> [RectF; N] {
    let total: f32 = weights.iter().sum();
    let gap = gap.min(r.height() / (2 * N) as f32);
    let space = (r.height() - gap * (N - 1) as f32).max(0.0);
    let mut y = r.top;
    std::array::from_fn(|i| {
        let h = space * weights[i] / total;
        let part = RectF::new(r.left, y, r.right, y + h);
        y += h + gap;
        part
    })
}

/// Splits `r` left to right by `weights`, with `gap` between parts.
pub fn split_h<const N: usize>(r: RectF, weights: &[f32; N], gap: f32) -> [RectF; N] {
    let total: f32 = weights.iter().sum();
    let gap = gap.min(r.width() / (2 * N) as f32);
    let space = (r.width() - gap * (N - 1) as f32).max(0.0);
    let mut x = r.left;
    std::array::from_fn(|i| {
        let w = space * weights[i] / total;
        let part = RectF::new(x, r.top, x + w, r.bottom);
        x += w + gap;
        part
    })
}

/// `n` equal columns across `r`.
pub fn columns(r: RectF, n: usize, gap: f32) -> Vec<RectF> {
    if n == 0 {
        return Vec::new();
    }
    let w = ((r.width() - gap * (n - 1) as f32) / n as f32).max(0.0);
    (0..n)
        .map(|i| {
            let x = r.left + i as f32 * (w + gap);
            RectF::new(x, r.top, x + w, r.bottom)
        })
        .collect()
}
