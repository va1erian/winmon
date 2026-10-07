//! NVIDIA GPU temperature through NVML (feature `nvml`).

use nvml_wrapper::Nvml;
use nvml_wrapper::enum_wrappers::device::TemperatureSensor;

use super::{TempReading, TempSource};

pub struct NvmlSource {
    nvml: Nvml,
}

impl NvmlSource {
    /// `None` (logged once) when there is no NVIDIA GPU or driver.
    pub fn new() -> Option<Self> {
        match Nvml::init() {
            Ok(nvml) => Some(NvmlSource { nvml }),
            Err(e) => {
                crate::log::info(&format!("NVML unavailable: {e}"));
                None
            }
        }
    }
}

impl TempSource for NvmlSource {
    fn read(&mut self) -> anyhow::Result<Vec<TempReading>> {
        let device = self.nvml.device_by_index(0)?;
        let t = device.temperature(TemperatureSensor::Gpu)?;
        Ok(vec![TempReading {
            label: "GPU".into(),
            celsius: Some(t as f32),
        }])
    }
}
