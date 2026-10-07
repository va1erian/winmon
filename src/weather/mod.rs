//! Weather forecast, fetched on a worker thread (PLAN.md Phase 5).

pub mod icons;
pub mod openmeteo;

use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender, channel};
use std::time::Duration;

use crate::config::Weather;

#[derive(Debug, Clone, PartialEq)]
pub struct Forecast {
    pub current_temp: Option<f32>,
    pub current_code: Option<u8>,
    pub days: Vec<Day>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Day {
    /// 0 = Sunday.
    pub weekday: Option<u8>,
    /// WMO weather code.
    pub code: Option<u8>,
    pub max: Option<f32>,
    pub min: Option<f32>,
    pub rain_pct: Option<f32>,
}

/// Retry delays after consecutive failures, capped at the normal interval.
const BACKOFF: [Duration; 4] = [
    Duration::from_secs(60),
    Duration::from_secs(120),
    Duration::from_secs(300),
    Duration::from_secs(600),
];

/// A handle to the worker. Dropping it stops the worker at its next wake.
pub struct WeatherWorker {
    wake: Sender<()>,
}

impl WeatherWorker {
    /// Fetches now instead of waiting for the next interval (e.g. after resume).
    pub fn refresh(&self) {
        let _ = self.wake.send(());
    }
}

/// Spawns the weather worker: fetch on start, then every `interval`, with
/// backoff on failure. `report` runs on the worker thread; when it returns
/// `false` (the window is gone) the worker exits.
pub fn spawn(
    config: &Weather,
    interval: Duration,
    report: impl Fn(Result<Forecast, String>) -> bool + Send + 'static,
) -> WeatherWorker {
    let (wake, rx) = channel();
    let url = openmeteo::url(config);
    let _ = std::thread::Builder::new()
        .name("weather".into())
        .stack_size(256 * 1024)
        .spawn(move || run(&url, interval, &rx, report));
    WeatherWorker { wake }
}

fn run(
    url: &str,
    interval: Duration,
    wake: &Receiver<()>,
    report: impl Fn(Result<Forecast, String>) -> bool,
) {
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(15)))
        .build()
        .into();
    let mut failures = 0usize;
    loop {
        let result = fetch(&agent, url).map_err(|e| format!("{e:#}"));
        let delay = match &result {
            Ok(_) => {
                failures = 0;
                interval
            }
            Err(_) => {
                let d = BACKOFF[failures.min(BACKOFF.len() - 1)].min(interval);
                failures += 1;
                d
            }
        };
        if !report(result) {
            return;
        }
        match wake.recv_timeout(delay) {
            Ok(()) | Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => return,
        }
        // Collapse a burst of refresh requests into one fetch.
        while wake.try_recv().is_ok() {}
    }
}

fn fetch(agent: &ureq::Agent, url: &str) -> anyhow::Result<Forecast> {
    let body = agent.get(url).call()?.body_mut().read_to_string()?;
    openmeteo::parse(&body)
}
