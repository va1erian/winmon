//! Cheap synchronous samplers that run on the UI tick.

pub mod cpu;
pub mod memory;

use windows::Win32::System::SystemInformation::GetLocalTime;

use crate::model::LocalTime;

pub fn local_time() -> LocalTime {
    // SAFETY: GetLocalTime has no preconditions and returns by value.
    let t = unsafe { GetLocalTime() };
    LocalTime {
        year: t.wYear,
        month: t.wMonth as u8,
        day: t.wDay as u8,
        weekday: t.wDayOfWeek as u8,
        hour: t.wHour as u8,
        minute: t.wMinute as u8,
        second: t.wSecond as u8,
    }
}
