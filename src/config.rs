//! The TOML config (PLAN.md §3). Every key has a default, so an empty file —
//! or no file at all — works.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    pub display: Display,
    pub refresh: Refresh,
    pub temps: Temps,
    pub weather: Weather,
    pub clock: Clock,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct Display {
    /// Substring of the monitor's friendly name, case-insensitive.
    pub monitor: Option<String>,
    /// Exact device name, e.g. `\\.\DISPLAY3`.
    pub monitor_device: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct Refresh {
    pub tick_ms: u32,
    pub history_seconds: usize,
    pub temps_ms: u64,
    pub weather_minutes: u64,
}

impl Default for Refresh {
    fn default() -> Self {
        Refresh {
            tick_ms: 1000,
            history_seconds: 120,
            temps_ms: 2000,
            weather_minutes: 30,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum TempSourceKind {
    Lhm,
    None,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct Temps {
    pub source: TempSourceKind,
    pub lhm_url: String,
    /// LHM sensor id for the CPU tile, as printed by `--list-sensors`.
    pub cpu_sensor: Option<String>,
    /// LHM sensor id for the GPU tile.
    pub gpu_sensor: Option<String>,
    /// Read the NVIDIA GPU directly; overrides `gpu_sensor` (feature `nvml`).
    pub nvml: bool,
    /// Value colour thresholds, °C.
    pub warm: f32,
    pub hot: f32,
}

impl Default for Temps {
    fn default() -> Self {
        Temps {
            source: TempSourceKind::Lhm,
            lhm_url: "http://127.0.0.1:8085/data.json".into(),
            cpu_sensor: None,
            gpu_sensor: None,
            nvml: true,
            warm: 60.0,
            hot: 80.0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Units {
    Metric,
    Imperial,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct Weather {
    pub enabled: bool,
    pub latitude: f64,
    pub longitude: f64,
    pub days: u32,
    pub units: Units,
}

impl Default for Weather {
    fn default() -> Self {
        // Paris: a visible default until the user sets their location.
        Weather {
            enabled: true,
            latitude: 48.8566,
            longitude: 2.3522,
            days: 5,
            units: Units::Metric,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct Clock {
    pub format_24h: bool,
    pub show_seconds: bool,
}

impl Default for Clock {
    fn default() -> Self {
        Clock {
            format_24h: true,
            show_seconds: false,
        }
    }
}

impl Config {
    pub fn parse(text: &str) -> anyhow::Result<Config> {
        let mut config: Config = toml::from_str(text)?;
        config.clamp();
        Ok(config)
    }

    /// Pulls every value into its supported range.
    pub fn clamp(&mut self) {
        let r = &mut self.refresh;
        r.tick_ms = r.tick_ms.clamp(100, 60_000);
        r.history_seconds = r.history_seconds.clamp(2, 3600);
        r.temps_ms = r.temps_ms.clamp(250, 600_000);
        r.weather_minutes = r.weather_minutes.clamp(5, 24 * 60);
        let w = &mut self.weather;
        w.days = w.days.clamp(1, 16);
        w.latitude = w.latitude.clamp(-90.0, 90.0);
        w.longitude = w.longitude.clamp(-180.0, 180.0);
        for name in [&mut self.display.monitor, &mut self.display.monitor_device] {
            if name.as_deref().is_some_and(|n| n.trim().is_empty()) {
                *name = None;
            }
        }
    }

    /// The config as TOML, with a header saying where it came from. Unset
    /// optional keys are left out, so they keep meaning "automatic".
    pub fn to_toml(&self) -> String {
        let body = toml::to_string_pretty(self).expect("config serializes");
        format!(
            "# winmon config, written by `winmon --settings`. Every key is optional;\n\
             # see winmon.example.toml for what each one does.\n\n{body}"
        )
    }

    /// Writes the config to `path` atomically (temp file + rename), so a
    /// running winmon watching the file never reads half of it.
    pub fn save(&self, path: &Path) -> anyhow::Result<()> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let tmp = path.with_extension("toml.tmp");
        std::fs::write(&tmp, self.to_toml())?;
        std::fs::rename(&tmp, path)?;
        Ok(())
    }

    /// Loads `path`, or the default location. A missing default file is not an
    /// error; a missing explicit `--config` file is.
    pub fn load(path: Option<&Path>) -> anyhow::Result<Config> {
        let (path, explicit) = match path {
            Some(p) => (p.to_path_buf(), true),
            None => match default_path() {
                Some(p) => (p, false),
                None => return Ok(Config::default()),
            },
        };
        match std::fs::read_to_string(&path) {
            Ok(text) => {
                Config::parse(&text).map_err(|e| anyhow::anyhow!("{}: {e}", path.display()))
            }
            Err(e) if !explicit && e.kind() == std::io::ErrorKind::NotFound => {
                Ok(Config::default())
            }
            Err(e) => Err(anyhow::anyhow!("{}: {e}", path.display())),
        }
    }
}

/// `%APPDATA%\winmon\winmon.toml`.
pub fn default_path() -> Option<PathBuf> {
    std::env::var_os("APPDATA").map(|d| PathBuf::from(d).join("winmon").join("winmon.toml"))
}
