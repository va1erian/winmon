//! Place search through Open-Meteo's geocoding API, for the settings dialog:
//! the user types a city instead of looking up its coordinates.

use std::time::Duration;

use serde::Deserialize;

const URL: &str = "https://geocoding-api.open-meteo.com/v1/search";

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Place {
    pub name: String,
    pub latitude: f64,
    pub longitude: f64,
    #[serde(default)]
    pub country: Option<String>,
    /// State or region.
    #[serde(default)]
    pub admin1: Option<String>,
}

impl Place {
    /// "Paris, Île-de-France, France".
    pub fn label(&self) -> String {
        [
            Some(&self.name),
            self.admin1.as_ref(),
            self.country.as_ref(),
        ]
        .into_iter()
        .flatten()
        .filter(|s| !s.is_empty())
        .cloned()
        .collect::<Vec<_>>()
        .join(", ")
    }
}

#[derive(Deserialize)]
struct Response {
    // Absent when nothing matches.
    #[serde(default)]
    results: Vec<Place>,
}

pub fn parse(json: &str) -> anyhow::Result<Vec<Place>> {
    Ok(serde_json::from_str::<Response>(json)?.results)
}

/// Up to 8 places matching `name`. Blocking: call it off the UI thread.
pub fn search(name: &str) -> anyhow::Result<Vec<Place>> {
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(10)))
        .build()
        .into();
    let body = agent
        .get(URL)
        .query("name", name.trim())
        .query("count", "8")
        .query("language", "en")
        .query("format", "json")
        .call()?
        .body_mut()
        .read_to_string()?;
    parse(&body)
}
