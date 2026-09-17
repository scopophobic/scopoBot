use anyhow::{bail, Result};
use companion_core::Weather;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct City {
    pub name: String,
    pub country: String,
    pub latitude: f64,
    pub longitude: f64,
}
#[derive(Deserialize)]
struct Search {
    #[serde(default)]
    results: Vec<City>,
}

fn agent() -> ureq::Agent {
    let builder =
        ureq::Agent::config_builder().timeout_global(Some(std::time::Duration::from_secs(12)));
    #[cfg(target_os = "windows")]
    let builder = builder.tls_config(
        ureq::tls::TlsConfig::builder()
            .provider(ureq::tls::TlsProvider::NativeTls)
            .build(),
    );
    builder.build().into()
}
pub fn search(query: &str) -> Result<Vec<City>> {
    if query.trim().chars().count() < 2 {
        bail!("Enter at least two letters of a city name");
    }
    let result: Search = agent()
        .get("https://geocoding-api.open-meteo.com/v1/search")
        .query("name", query.trim())
        .query("count", "5")
        .query("language", "en")
        .call()?
        .body_mut()
        .read_json()?;
    Ok(result.results)
}
pub fn fetch(city: &City) -> Result<Weather> {
    if !city.latitude.is_finite()
        || !city.longitude.is_finite()
        || city.latitude.abs() > 90.
        || city.longitude.abs() > 180.
    {
        bail!("Invalid city coordinates");
    }
    let value: serde_json::Value = agent()
        .get("https://api.open-meteo.com/v1/forecast")
        .query("latitude", city.latitude.to_string())
        .query("longitude", city.longitude.to_string())
        .query("current", "weather_code,temperature_2m")
        .call()?
        .body_mut()
        .read_json()?;
    let current = &value["current"];
    let code = current["weather_code"]
        .as_u64()
        .ok_or_else(|| anyhow::anyhow!("Weather response has no condition"))?;
    Ok(match code {
        95..=99 => Weather::Thunder,
        71..=77 | 85..=86 => Weather::Snow,
        51..=67 | 80..=82 => Weather::Rain,
        1..=3 | 45..=48 => Weather::Cloudy,
        _ if current["temperature_2m"].as_f64().is_some_and(|v| v > 30.) => Weather::Hot,
        _ if current["temperature_2m"].as_f64().is_some_and(|v| v < 5.) => Weather::Cold,
        _ => Weather::Clear,
    })
}
