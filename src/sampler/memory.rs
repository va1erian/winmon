//! Physical memory from `GlobalMemoryStatusEx`.

use windows::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};

use crate::model::Memory;

pub fn sample() -> Option<Memory> {
    let mut status = MEMORYSTATUSEX {
        dwLength: size_of::<MEMORYSTATUSEX>() as u32,
        ..Default::default()
    };
    // SAFETY: `status` is a properly sized MEMORYSTATUSEX with dwLength set.
    unsafe { GlobalMemoryStatusEx(&mut status) }.ok()?;
    Some(Memory {
        used_bytes: status.ullTotalPhys.saturating_sub(status.ullAvailPhys),
        total_bytes: status.ullTotalPhys,
    })
}
