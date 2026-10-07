//! The dashboard: one Direct2D custom widget filling the window.

mod clock;
mod graph;
mod icons;
pub mod layout;
pub mod palette;
mod temps;
mod weather;

use std::cell::RefCell;

use win32ui::d2d::{D2dCanvas, Font, FontSpec, PointF, RectF, TextSystem};
use win32ui::gdi::Canvas;
use win32ui::{Color, CustomWidget, Input, Key, Rect, Renderer, Theme, WidgetCx};

use crate::config::{Config, Units};
use crate::model::Model;

/// What the dashboard asks of the app.
pub enum Event {
    Quit,
}

/// Display options taken from the config once.
pub struct Options {
    pub format_24h: bool,
    pub show_seconds: bool,
    pub warm: f32,
    pub hot: f32,
    pub temp_unit: &'static str,
    pub weather_enabled: bool,
}

impl Options {
    pub fn from_config(c: &Config) -> Self {
        Options {
            format_24h: c.clock.format_24h,
            show_seconds: c.clock.show_seconds,
            warm: c.temps.warm,
            hot: c.temps.hot,
            temp_unit: match c.weather.units {
                Units::Metric => "°C",
                Units::Imperial => "°F",
            },
            weather_enabled: c.weather.enabled,
        }
    }
}

/// Fonts for one scale. Rebuilt only when the scale changes.
pub(crate) struct Fonts {
    unit: f32,
    pub clock: Font,
    pub big: Font,
    pub value: Font,
    pub label: Font,
    pub small: Font,
}

const FAMILY: &str = "Segoe UI Variable Display";

impl Fonts {
    fn new(unit: f32) -> Option<Fonts> {
        let system = TextSystem::new().ok()?;
        let font = |size: f32, weight: u16| {
            system
                .font(&FontSpec::new(FAMILY, size * unit).weight(weight))
                .ok()
        };
        Some(Fonts {
            unit,
            clock: font(84.0, 300)?,
            big: font(34.0, 400)?,
            value: font(26.0, 600)?,
            label: font(15.0, 600)?,
            small: font(13.0, 400)?,
        })
    }
}

pub struct Dashboard {
    pub model: Model,
    pub options: Options,
    fonts: RefCell<Option<Fonts>>,
}

impl Dashboard {
    pub fn new(model: Model, options: Options) -> Self {
        Dashboard {
            model,
            options,
            fonts: RefCell::new(None),
        }
    }
}

impl CustomWidget for Dashboard {
    type Event = Event;

    fn paint(&self, canvas: &Canvas, bounds: Rect, _theme: &Theme) {
        // Only reached if Direct2D is unavailable.
        canvas.fill_rect(bounds, palette::BACKGROUND);
    }

    fn renderer(&self) -> Renderer {
        Renderer::Direct2D
    }

    fn paint_d2d(&self, canvas: &mut D2dCanvas<'_>, bounds: RectF, _theme: &Theme) {
        canvas.clear(palette::BACKGROUND);
        let unit = layout::unit(bounds);
        let mut fonts = self.fonts.borrow_mut();
        if fonts.as_ref().is_none_or(|f| (f.unit - unit).abs() > 0.001) {
            *fonts = Fonts::new(unit);
        }
        let Some(fonts) = fonts.as_ref() else { return };
        let s = layout::sections(bounds);
        let m = &self.model;
        let o = &self.options;

        clock::draw(canvas, s.clock, fonts, unit, &m.now, o);
        let cpu = m.cpu.latest();
        graph::draw(
            canvas,
            s.cpu,
            fonts,
            unit,
            graph::Metric {
                title: "CPU",
                detail: String::new(),
                value: cpu,
                history: &m.cpu,
                accent: palette::CPU,
            },
        );
        let gib = |b: u64| b as f64 / (1u64 << 30) as f64;
        graph::draw(
            canvas,
            s.mem,
            fonts,
            unit,
            graph::Metric {
                title: "MEMORY",
                detail: format!(
                    "{:.1} / {:.1} GB",
                    gib(m.memory.used_bytes),
                    gib(m.memory.total_bytes)
                ),
                value: m.mem.latest(),
                history: &m.mem,
                accent: palette::MEM,
            },
        );
        temps::draw(canvas, s.temps, fonts, unit, m, o);
        if o.weather_enabled {
            weather::draw(canvas, s.weather, fonts, unit, m, o);
        }
    }

    fn input(&self, input: Input, cx: &mut WidgetCx<Event>) {
        if let Input::KeyDown {
            key: Key::ESCAPE, ..
        } = input
        {
            cx.emit(Event::Quit);
        }
    }
}

// --- small text helpers shared by the sections ---

#[derive(Clone, Copy)]
pub(crate) enum Align {
    Left,
    Center,
    Right,
}

/// Draws one line of `text` horizontally aligned in `[left, right]`, its top
/// at `top`. Returns the line's height.
#[allow(clippy::too_many_arguments)]
pub(crate) fn text(
    canvas: &mut D2dCanvas<'_>,
    font: &Font,
    text: &str,
    left: f32,
    right: f32,
    top: f32,
    align: Align,
    color: Color,
) -> f32 {
    let Ok(layout) = font.layout(text, f32::MAX) else {
        return 0.0;
    };
    let w = layout.width();
    let x = match align {
        Align::Left => left,
        Align::Center => left + ((right - left) - w) / 2.0,
        Align::Right => right - w,
    };
    canvas.draw_text(&layout, PointF::new(x, top), color);
    layout.height()
}

/// Like [`text`], vertically centred on `center_y`.
#[allow(clippy::too_many_arguments)]
pub(crate) fn text_vcenter(
    canvas: &mut D2dCanvas<'_>,
    font: &Font,
    s: &str,
    left: f32,
    right: f32,
    center_y: f32,
    align: Align,
    color: Color,
) {
    let h = font.metrics().line_height();
    text(
        canvas,
        font,
        s,
        left,
        right,
        center_y - h / 2.0,
        align,
        color,
    );
}
