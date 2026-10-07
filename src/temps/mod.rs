//! Temperatures, read on a worker thread (PLAN.md Phase 4).

pub mod lhm;
#[cfg(feature = "nvml")]
pub mod nvml;

use std::time::Duration;

use crate::config::{TempSourceKind, Temps};

#[derive(Debug, Clone, PartialEq)]
pub struct TempReading {
    pub label: String,
    /// `None` when the sensor is configured but currently missing.
    pub celsius: Option<f32>,
}

pub trait TempSource: Send {
    fn read(&mut self) -> anyhow::Result<Vec<TempReading>>;
}

/// Every source the config asks for, read in order and concatenated.
struct Combined {
    sources: Vec<Box<dyn TempSource>>,
}

impl TempSource for Combined {
    fn read(&mut self) -> anyhow::Result<Vec<TempReading>> {
        let mut all = Vec::new();
        let mut first_err = None;
        for source in &mut self.sources {
            match source.read() {
                Ok(r) => all.extend(r),
                Err(e) => {
                    first_err.get_or_insert(e);
                }
            }
        }
        match first_err {
            Some(e) if all.iter().all(|r| r.celsius.is_none()) => Err(e),
            _ => Ok(all),
        }
    }
}

fn build(config: &Temps) -> Option<Box<dyn TempSource>> {
    let mut sources: Vec<Box<dyn TempSource>> = Vec::new();
    #[allow(unused_mut)]
    let mut want_lhm_gpu = true;
    #[cfg(feature = "nvml")]
    if config.nvml
        && let Some(gpu) = nvml::NvmlSource::new()
    {
        sources.push(Box::new(gpu));
        want_lhm_gpu = false;
    }
    if config.source == TempSourceKind::Lhm {
        sources.insert(0, Box::new(lhm::LhmSource::new(config, want_lhm_gpu)));
    }
    (!sources.is_empty()).then(|| Box::new(Combined { sources }) as Box<dyn TempSource>)
}

/// Spawns the temperature worker. `report` runs on the worker thread for each
/// read; when it returns `false` (the window is gone) the worker exits.
pub fn spawn(
    config: &Temps,
    interval: Duration,
    report: impl Fn(Result<Vec<TempReading>, String>) -> bool + Send + 'static,
) {
    let Some(mut source) = build(config) else {
        return;
    };
    let _ = std::thread::Builder::new()
        .name("temps".into())
        .stack_size(256 * 1024)
        .spawn(move || {
            loop {
                let result = source.read().map_err(|e| format!("{e:#}"));
                if !report(result) {
                    break;
                }
                std::thread::sleep(interval);
            }
        });
}
