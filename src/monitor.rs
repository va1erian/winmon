//! Which monitor the dashboard goes on.

use win32ui::MonitorInfo;

use crate::config::Display;

/// The monitor to go fullscreen on, or `None` to stay windowed (only the
/// primary is attached and the config didn't ask for it — e.g. the panel is
/// unplugged).
///
/// Order: `monitor_device` exact match, then `monitor` as a case-insensitive
/// substring of the friendly name, then the smallest non-primary monitor.
pub fn select<'a>(monitors: &'a [MonitorInfo], display: &Display) -> Option<&'a MonitorInfo> {
    if let Some(device) = display.monitor_device.as_deref().filter(|d| !d.is_empty())
        && let Some(m) = monitors
            .iter()
            .find(|m| m.device_name.eq_ignore_ascii_case(device))
    {
        return Some(m);
    }
    if let Some(name) = display
        .monitor
        .as_deref()
        .filter(|n| !n.is_empty())
        .map(str::to_lowercase)
        && let Some(m) = monitors
            .iter()
            .find(|m| m.friendly_name.to_lowercase().contains(&name))
    {
        return Some(m);
    }
    monitors
        .iter()
        .filter(|m| !m.primary)
        .min_by_key(|m| i64::from(m.rect.width()) * i64::from(m.rect.height()))
}

pub fn describe(m: &MonitorInfo) -> String {
    format!(
        "{:<14} {:<28} {:>5}x{:<5} at ({},{}){}  dpi {}",
        m.device_name,
        if m.friendly_name.is_empty() {
            "(no name)"
        } else {
            &m.friendly_name
        },
        m.rect.width(),
        m.rect.height(),
        m.rect.left,
        m.rect.top,
        if m.primary { "  primary" } else { "" },
        m.dpi
    )
}
