//! Open-Meteo forecast request and response types.

use serde::Deserialize;

use super::{Day, Forecast};
use crate::config::{Units, Weather};

pub fn url(config: &Weather) -> String {
    let mut url = format!(
        "https://api.open-meteo.com/v1/forecast?latitude={}&longitude={}\
         &current=temperature_2m,weather_code\
         &daily=weather_code,temperature_2m_max,temperature_2m_min,precipitation_probability_max\
         &timezone=auto&forecast_days={}",
        config.latitude, config.longitude, config.days
    );
    if config.units == Units::Imperial {
        url.push_str("&temperature_unit=fahrenheit");
    }
    url
}

#[derive(Debug, Deserialize)]
pub struct Response {
    pub current: Option<Current>,
    pub daily: Daily,
}

#[derive(Debug, Deserialize)]
pub struct Current {
    pub temperature_2m: Option<f32>,
    pub weather_code: Option<u8>,
}

#[derive(Debug, Deserialize)]
pub struct Daily {
    pub time: Vec<String>,
    #[serde(default)]
    pub weather_code: Vec<Option<u8>>,
    #[serde(default)]
    pub temperature_2m_max: Vec<Option<f32>>,
    #[serde(default)]
    pub temperature_2m_min: Vec<Option<f32>>,
    #[serde(default)]
    pub precipitation_probability_max: Vec<Option<f32>>,
}

pub fn parse(json: &str) -> anyhow::Result<Forecast> {
    let response: Response = serde_json::from_str(json)?;
    Ok(response.into())
}

impl From<Response> for Forecast {
    fn from(r: Response) -> Forecast {
        let d = &r.daily;
        let at = |v: &Vec<Option<f32>>, i: usize| v.get(i).copied().flatten();
        let days = d
            .time
            .iter()
            .enumerate()
            .map(|(i, date)| Day {
                weekday: weekday_of(date),
                code: d.weather_code.get(i).copied().flatten(),
                max: at(&d.temperature_2m_max, i),
                min: at(&d.temperature_2m_min, i),
                rain_pct: at(&d.precipitation_probability_max, i),
            })
            .collect();
        Forecast {
            current_temp: r.current.as_ref().and_then(|c| c.temperature_2m),
            current_code: r.current.as_ref().and_then(|c| c.weather_code),
            days,
        }
    }
}

/// Day of the week (0 = Sunday) of a `YYYY-MM-DD` date (Sakamoto's method).
pub fn weekday_of(date: &str) -> Option<u8> {
    let mut parts = date.splitn(3, '-').map(|p| p.parse::<i32>().ok());
    let (mut y, m, d) = (parts.next()??, parts.next()??, parts.next()??);
    if !(1..=12).contains(&m) || !(1..=31).contains(&d) {
        return None;
    }
    const T: [i32; 12] = [0, 3, 2, 5, 0, 3, 5, 1, 4, 6, 2, 4];
    if m < 3 {
        y -= 1;
    }
    Some(((y + y / 4 - y / 100 + y / 400 + T[(m - 1) as usize] + d).rem_euclid(7)) as u8)
}
