//! Time and date.

use win32ui::d2d::{D2dCanvas, RectF};

use super::{Align, Fonts, Options, palette, text};
use crate::model::LocalTime;

pub const WEEKDAYS: [&str; 7] = [
    "Sunday",
    "Monday",
    "Tuesday",
    "Wednesday",
    "Thursday",
    "Friday",
    "Saturday",
];
const MONTHS: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];

pub fn time_string(t: &LocalTime, o: &Options) -> String {
    let (hour, suffix) = if o.format_24h {
        (t.hour, "")
    } else {
        let h = t.hour % 12;
        (
            if h == 0 { 12 } else { h },
            if t.hour < 12 { " AM" } else { " PM" },
        )
    };
    let mut s = if o.format_24h {
        format!("{hour:02}:{:02}", t.minute)
    } else {
        format!("{hour}:{:02}", t.minute)
    };
    if o.show_seconds {
        s.push_str(&format!(":{:02}", t.second));
    }
    s.push_str(suffix);
    s
}

pub fn date_string(t: &LocalTime) -> String {
    let weekday = WEEKDAYS.get(t.weekday as usize).copied().unwrap_or("");
    let month = MONTHS
        .get((t.month as usize).wrapping_sub(1))
        .copied()
        .unwrap_or("");
    format!("{weekday} {} {month} {}", t.day, t.year)
}

pub fn draw(
    canvas: &mut D2dCanvas<'_>,
    r: RectF,
    fonts: &Fonts,
    unit: f32,
    t: &LocalTime,
    o: &Options,
) {
    let time = time_string(t, o);
    let date = date_string(t);
    let time_h = fonts.clock.metrics().line_height();
    let date_h = fonts.big.metrics().line_height() * 0.6;
    let total = time_h + date_h;
    let top = r.top + ((r.height() - total) / 2.0).max(0.0) - 4.0 * unit;
    text(
        canvas,
        &fonts.clock,
        &time,
        r.left,
        r.right,
        top,
        Align::Center,
        palette::TEXT,
    );
    text(
        canvas,
        &fonts.label,
        &date,
        r.left,
        r.right,
        top + time_h - 4.0 * unit,
        Align::Center,
        palette::TEXT_DIM,
    );
}
