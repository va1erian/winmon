//! LibreHardwareMonitor's "Remote Web Server" JSON (`/data.json`).
//!
//! The tree is `{ Text, Children: [...] }` nodes; leaf sensors also carry
//! `SensorId` (e.g. `/amdcpu/0/temperature/2`), `Type` and `Value`
//! (`"45.0 °C"`, or `"45,0 °C"` under a comma-decimal locale). Every field is
//! read defensively: versions differ.

use std::time::Duration;

use serde_json::Value;

use super::{TempReading, TempSource};
use crate::config::Temps;

#[derive(Debug, Clone, PartialEq)]
pub struct Sensor {
    pub id: String,
    /// The sensor's own name, e.g. `"Core (Tctl/Tdie)"`.
    pub name: String,
    /// The hardware node it hangs under, e.g. `"AMD Ryzen 7 5800X"`.
    pub hardware: String,
    pub kind: String,
    pub value: Option<f32>,
}

/// Every sensor in the tree, depth-first.
pub fn sensors(root: &Value) -> Vec<Sensor> {
    let mut out = Vec::new();
    walk(root, 0, "", &mut out);
    out
}

/// Depth 0 is the root ("Sensor"), 1 the machine, 2 the hardware, 3 the
/// sensor groups ("Temperatures"), 4 the sensors. Hardware can nest deeper
/// (e.g. a mainboard's Super I/O chip), so the name is taken from whichever
/// ancestor is not a group.
fn walk(node: &Value, depth: usize, hardware: &str, out: &mut Vec<Sensor>) {
    let text = node.get("Text").and_then(Value::as_str).unwrap_or("");
    if let Some(id) = node.get("SensorId").and_then(Value::as_str) {
        out.push(Sensor {
            id: id.to_string(),
            name: text.to_string(),
            hardware: hardware.to_string(),
            kind: node
                .get("Type")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string(),
            value: node
                .get("Value")
                .and_then(Value::as_str)
                .and_then(parse_value),
        });
        return;
    }
    let hardware = if depth >= 2 && !is_group_name(text) {
        text
    } else {
        hardware
    };
    if let Some(children) = node.get("Children").and_then(Value::as_array) {
        for child in children {
            walk(child, depth + 1, hardware, out);
        }
    }
}

fn is_group_name(text: &str) -> bool {
    matches!(
        text,
        "Voltages"
            | "Clocks"
            | "Temperatures"
            | "Load"
            | "Fans"
            | "Controls"
            | "Powers"
            | "Data"
            | "SmallData"
            | "Throughput"
            | "Levels"
            | "Factors"
            | "Currents"
            | "Energy"
            | "Noise"
            | "Timing"
    )
}

/// `"45.0 °C"`, `"45,0 °C"`, `"45 °C"` → 45.0, independent of the locale.
pub fn parse_value(text: &str) -> Option<f32> {
    let numeric: String = text
        .trim()
        .chars()
        .take_while(|c| c.is_ascii_digit() || matches!(c, '.' | ',' | '-' | '+'))
        .map(|c| if c == ',' { '.' } else { c })
        .collect();
    numeric.parse().ok()
}

pub fn is_temperature(sensor: &Sensor) -> bool {
    sensor.kind.eq_ignore_ascii_case("Temperature") || sensor.id.contains("/temperature/")
}

/// The CPU package sensor, when none is configured.
pub fn guess_cpu(sensors: &[Sensor]) -> Option<&Sensor> {
    let cpu = |s: &&Sensor| is_temperature(s) && s.id.contains("cpu/");
    ["Package", "Tctl", "Tdie", "Core Average", "Core Max"]
        .iter()
        .find_map(|p| sensors.iter().filter(cpu).find(|s| s.name.contains(p)))
        .or_else(|| sensors.iter().find(cpu))
}

/// The GPU core sensor, when none is configured.
pub fn guess_gpu(sensors: &[Sensor]) -> Option<&Sensor> {
    let gpu = |s: &&Sensor| is_temperature(s) && s.id.starts_with("/gpu");
    sensors
        .iter()
        .filter(gpu)
        .find(|s| s.name.contains("Core"))
        .or_else(|| sensors.iter().find(gpu))
}

pub fn agent() -> ureq::Agent {
    ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(2)))
        .build()
        .into()
}

pub fn fetch(agent: &ureq::Agent, url: &str) -> anyhow::Result<Value> {
    Ok(agent.get(url).call()?.body_mut().read_json::<Value>()?)
}

pub struct LhmSource {
    agent: ureq::Agent,
    url: String,
    cpu: Option<String>,
    gpu: Option<String>,
    want_gpu: bool,
}

impl LhmSource {
    pub fn new(config: &Temps, want_gpu: bool) -> Self {
        LhmSource {
            agent: agent(),
            url: config.lhm_url.clone(),
            cpu: config.cpu_sensor.clone(),
            gpu: config.gpu_sensor.clone(),
            want_gpu,
        }
    }

    /// The CPU and GPU readings from a parsed tree. A configured id that is
    /// missing yields a `None` reading (shown as "—"); with no id configured
    /// the best guess is used, and nothing is shown if there is none.
    pub fn readings(&self, all: &[Sensor]) -> Vec<TempReading> {
        let pick = |id: &Option<String>, guess: fn(&[Sensor]) -> Option<&Sensor>| match id {
            Some(id) => Some(all.iter().find(|s| &s.id == id).and_then(|s| s.value)),
            None => guess(all).map(|s| s.value),
        };
        let mut out = Vec::new();
        if let Some(celsius) = pick(&self.cpu, guess_cpu) {
            out.push(TempReading {
                label: "CPU".into(),
                celsius,
            });
        }
        if self.want_gpu
            && let Some(celsius) = pick(&self.gpu, guess_gpu)
        {
            out.push(TempReading {
                label: "GPU".into(),
                celsius,
            });
        }
        out
    }
}

impl TempSource for LhmSource {
    fn read(&mut self) -> anyhow::Result<Vec<TempReading>> {
        let json = fetch(&self.agent, &self.url)?;
        Ok(self.readings(&sensors(&json)))
    }
}
