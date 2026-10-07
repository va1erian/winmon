//! WMO weather interpretation codes → a handful of icon kinds.
//! The drawing lives in `ui::icons`.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IconKind {
    Clear,
    PartlyCloudy,
    Cloudy,
    Fog,
    Drizzle,
    Rain,
    Snow,
    Thunder,
}

pub fn kind(code: u8) -> IconKind {
    match code {
        0 | 1 => IconKind::Clear,
        2 => IconKind::PartlyCloudy,
        3 => IconKind::Cloudy,
        45 | 48 => IconKind::Fog,
        51..=57 => IconKind::Drizzle,
        61..=67 | 80..=82 => IconKind::Rain,
        71..=77 | 85 | 86 => IconKind::Snow,
        95..=99 => IconKind::Thunder,
        _ => IconKind::Cloudy,
    }
}
