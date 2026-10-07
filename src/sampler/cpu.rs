//! Total CPU usage from `GetSystemTimes` deltas.

use windows::Win32::Foundation::FILETIME;
use windows::Win32::System::Threading::GetSystemTimes;

#[derive(Debug, Default)]
pub struct CpuSampler {
    prev: Option<Times>,
}

#[derive(Debug, Clone, Copy)]
struct Times {
    idle: u64,
    kernel: u64,
    user: u64,
}

fn ticks(t: FILETIME) -> u64 {
    (u64::from(t.dwHighDateTime) << 32) | u64::from(t.dwLowDateTime)
}

impl CpuSampler {
    pub fn new() -> Self {
        CpuSampler::default()
    }

    /// Forgets the previous reading, e.g. after resume from sleep, where the
    /// first delta spans the whole sleep.
    pub fn reset(&mut self) {
        self.prev = None;
    }

    /// Usage in `0..=1` since the previous call; `None` on the first call.
    pub fn sample(&mut self) -> Option<f32> {
        let (mut idle, mut kernel, mut user) = Default::default();
        // SAFETY: three valid out-pointers to FILETIMEs on our stack.
        unsafe { GetSystemTimes(Some(&mut idle), Some(&mut kernel), Some(&mut user)) }.ok()?;
        let now = Times {
            idle: ticks(idle),
            kernel: ticks(kernel),
            user: ticks(user),
        };
        let prev = self.prev.replace(now)?;
        Some(usage(prev, now))
    }
}

/// Kernel time includes idle time, so busy = total − idle.
fn usage(prev: Times, now: Times) -> f32 {
    let idle = now.idle.saturating_sub(prev.idle);
    let total = now.kernel.saturating_sub(prev.kernel) + now.user.saturating_sub(prev.user);
    if total == 0 {
        return 0.0;
    }
    (1.0 - idle as f64 / total as f64).clamp(0.0, 1.0) as f32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn usage_math() {
        let a = Times {
            idle: 0,
            kernel: 0,
            user: 0,
        };
        let b = Times {
            idle: 75,
            kernel: 100,
            user: 0,
        };
        assert!((usage(a, b) - 0.25).abs() < 1e-6);
        let c = Times {
            idle: 75,
            kernel: 100,
            user: 100,
        };
        assert!((usage(b, c) - 1.0).abs() < 1e-6);
        assert_eq!(usage(c, c), 0.0);
    }
}
