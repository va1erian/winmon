//! The settings dialog's pure parts: drop-down items and field parsing.
//! Kept free of UI types so they can be unit-tested.

use std::str::FromStr;

use win32ui::MonitorInfo;

use crate::config::Display;
use crate::temps::lhm::{self, Sensor};

/// What the monitor drop-down stores in the config.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MonitorChoice {
    /// Neither key set: the smallest non-primary monitor.
    Auto,
    /// `monitor = "<friendly name>"`.
    Name(String),
    /// `monitor_device = "<device>"`, for monitors without a unique name.
    Device(String),
}

impl MonitorChoice {
    /// The `(monitor, monitor_device)` config pair.
    pub fn to_config(&self) -> (Option<String>, Option<String>) {
        match self {
            MonitorChoice::Auto => (None, None),
            MonitorChoice::Name(n) => (Some(n.clone()), None),
            MonitorChoice::Device(d) => (None, Some(d.clone())),
        }
    }
}

/// The drop-down items for the attached monitors, and the item matching the
/// current config. A configured monitor that isn't attached gets its own item
/// so saving doesn't silently drop it.
pub fn monitor_choices(
    monitors: &[MonitorInfo],
    display: &Display,
) -> (Vec<(String, MonitorChoice)>, MonitorChoice) {
    let mut items = vec![(
        "Automatic (smallest secondary monitor)".to_string(),
        MonitorChoice::Auto,
    )];
    let choice_for = |m: &MonitorInfo| {
        let unique = !m.friendly_name.is_empty()
            && monitors
                .iter()
                .filter(|o| o.friendly_name == m.friendly_name)
                .count()
                == 1;
        if unique {
            MonitorChoice::Name(m.friendly_name.clone())
        } else {
            MonitorChoice::Device(m.device_name.clone())
        }
    };
    for m in monitors {
        let name = if m.friendly_name.is_empty() {
            &m.device_name
        } else {
            &m.friendly_name
        };
        let primary = if m.primary { ", primary" } else { "" };
        items.push((
            format!("{name} ({}×{}{primary})", m.rect.width(), m.rect.height()),
            choice_for(m),
        ));
    }

    let device = display.monitor_device.as_deref().filter(|d| !d.is_empty());
    let name = display.monitor.as_deref().filter(|n| !n.is_empty());
    let by_device = device.and_then(|d| {
        monitors
            .iter()
            .find(|m| m.device_name.eq_ignore_ascii_case(d))
    });
    let by_name = name.and_then(|n| {
        let n = n.to_lowercase();
        monitors
            .iter()
            .find(|m| m.friendly_name.to_lowercase().contains(&n))
    });
    let current = match (by_device.or(by_name), device, name) {
        (Some(m), _, _) => choice_for(m),
        (None, Some(d), _) => {
            let c = MonitorChoice::Device(d.to_string());
            items.push((format!("{d} (not connected)"), c.clone()));
            c
        }
        (None, None, Some(n)) => {
            let c = MonitorChoice::Name(n.to_string());
            items.push((format!("{n} (not connected)"), c.clone()));
            c
        }
        (None, None, None) => MonitorChoice::Auto,
    };
    (items, current)
}

/// Sensor drop-down items: "Automatic", then every temperature sensor. A
/// configured id that LibreHardwareMonitor doesn't report is kept as an item.
pub fn sensor_choices(sensors: &[Sensor], current: Option<&str>) -> Vec<(String, Option<String>)> {
    let mut items = vec![("Automatic (best guess)".to_string(), None)];
    for s in sensors.iter().filter(|s| lhm::is_temperature(s)) {
        let value = s.value.map_or(String::new(), |v| format!(" — {v:.0} °C"));
        items.push((
            format!("{} / {}{value}", s.hardware, s.name),
            Some(s.id.clone()),
        ));
    }
    if let Some(id) = current
        && !items.iter().any(|(_, v)| v.as_deref() == Some(id))
    {
        items.push((format!("{id} (not found)"), Some(id.to_string())));
    }
    items
}

/// Parses a number field, accepting a comma as the decimal separator.
/// The error names the field and the accepted range.
pub fn number<T>(field: &str, text: &str, min: T, max: T) -> Result<T, String>
where
    T: FromStr + PartialOrd + std::fmt::Display + Copy,
{
    let text = text.trim().replace(',', ".");
    match text.parse::<T>() {
        Ok(v) if v >= min && v <= max => Ok(v),
        _ => Err(format!("{field} must be a number from {min} to {max}.")),
    }
}

/// Formats a coordinate without float noise (`48.8566`, not `48.856600000000004`).
pub fn coordinate(v: f64) -> String {
    let s = format!("{v:.4}");
    let s = s.trim_end_matches('0').trim_end_matches('.');
    if s == "-0" { "0".into() } else { s.into() }
}
