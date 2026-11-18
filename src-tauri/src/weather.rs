//! HOME's WEATHER widget: the weather where the member is, from Open-Meteo (open-meteo.com), a free
//! weather service that needs no account or key.
//!
//! Castle only ever sends two kinds of requests: a place NAME typed by the member (to find its
//! coordinates) and a pair of coordinates (to read the weather there). Nothing else about the member
//! or the computer is sent. The place is kept in Castle's own settings, only on this computer.
//! Readings are kept for ten minutes, so a busy HOME does not ask again and again.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Mutex;
use std::time::{Duration, Instant};

const FORECAST: &str = "https://api.open-meteo.com/v1/forecast";
const SEARCH: &str = "https://geocoding-api.open-meteo.com/v1/search";
/// How long a reading is reused before Castle asks again.
const KEEP: Duration = Duration::from_secs(10 * 60);
/// How many days the widget shows, today included.
const DAYS: usize = 5;

/// A place on the map, as the member chose it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Place {
    pub name: String,
    /// the region and country, to tell two towns of one name apart ("Selangor, Malaysia")
    #[serde(default)]
    pub region: String,
    pub lat: f64,
    pub lon: f64,
}

/// What the member chose for the widget: where, and metric or imperial.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct WeatherSettings {
    pub place: Option<Place>,
    /// "metric" (°C, km/h) or "imperial" (°F, mph)
    pub units: String,
}

impl Default for WeatherSettings {
    fn default() -> Self {
        Self { place: None, units: "metric".into() }
    }
}

/// One day of the forecast.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Day {
    /// "2026-11-18"
    pub date: String,
    pub icon: String,
    pub label: String,
    pub high: Option<f64>,
    pub low: Option<f64>,
    /// the chance of rain, percent
    pub rain: Option<u8>,
}

/// The weather now, with the days ahead.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Reading {
    pub place: Place,
    pub units: String,
    pub temp: f64,
    pub feels: Option<f64>,
    pub humidity: Option<u8>,
    pub wind: Option<f64>,
    /// the direction the wind comes from, degrees
    pub wind_dir: Option<u16>,
    pub is_day: bool,
    pub icon: String,
    pub label: String,
    pub days: Vec<Day>,
}

/// The icon and the words for a WMO weather code (the numbering Open-Meteo uses).
pub fn condition(code: u32) -> (&'static str, &'static str) {
    match code {
        0 => ("clear", "Clear sky"),
        1 => ("clear", "Mostly clear"),
        2 => ("partly", "Partly cloudy"),
        3 => ("cloud", "Overcast"),
        45 | 48 => ("fog", "Fog"),
        51 | 53 | 55 => ("rain", "Drizzle"),
        56 | 57 => ("rain", "Freezing drizzle"),
        61 => ("rain", "Light rain"),
        63 => ("rain", "Rain"),
        65 => ("rain", "Heavy rain"),
        66 | 67 => ("rain", "Freezing rain"),
        71 => ("snow", "Light snow"),
        73 => ("snow", "Snow"),
        75 => ("snow", "Heavy snow"),
        77 => ("snow", "Snow grains"),
        80 => ("rain", "Light showers"),
        81 => ("rain", "Showers"),
        82 => ("rain", "Violent showers"),
        85 | 86 => ("snow", "Snow showers"),
        95 => ("storm", "Thunderstorm"),
        96 | 99 => ("storm", "Thunderstorm with hail"),
        _ => ("cloud", "Cloudy"),
    }
}

fn units_of(text: &str) -> &'static str {
    if text.trim().eq_ignore_ascii_case("imperial") {
        "imperial"
    } else {
        "metric"
    }
}

/// A coordinate pair that is on the map, kept to two decimals (about a kilometre) so Castle never
/// holds or sends a more exact spot than the weather needs.
pub fn clean_coords(lat: f64, lon: f64) -> Option<(f64, f64)> {
    if !lat.is_finite() || !lon.is_finite() || !(-90.0..=90.0).contains(&lat) || !(-180.0..=180.0).contains(&lon) {
        return None;
    }
    let round = |v: f64| (v * 100.0).round() / 100.0;
    Some((round(lat), round(lon)))
}

/// A place the member may keep: a name, and coordinates on the map.
pub fn clean_place(place: Place) -> Option<Place> {
    let (lat, lon) = clean_coords(place.lat, place.lon)?;
    let name: String = place.name.trim().chars().filter(|c| !c.is_control()).take(60).collect();
    let region: String = place.region.trim().chars().filter(|c| !c.is_control()).take(80).collect();
    if name.is_empty() {
        return None;
    }
    Some(Place { name, region, lat, lon })
}

/// The places Open-Meteo found for a name.
pub fn parse_places(answer: &Value) -> Vec<Place> {
    answer
        .get("results")
        .and_then(Value::as_array)
        .map(|list| {
            list.iter()
                .filter_map(|item| {
                    let name = item.get("name")?.as_str()?.to_string();
                    let text = |key: &str| item.get(key).and_then(Value::as_str).unwrap_or("").trim().to_string();
                    let region = [text("admin1"), text("country")].into_iter().filter(|part| !part.is_empty() && *part != name).collect::<Vec<_>>().join(", ");
                    clean_place(Place { name, region, lat: item.get("latitude")?.as_f64()?, lon: item.get("longitude")?.as_f64()? })
                })
                .take(6)
                .collect()
        })
        .unwrap_or_default()
}

fn number(list: &Value, key: &str, index: usize) -> Option<f64> {
    list.get(key)?.as_array()?.get(index)?.as_f64()
}

/// The reading in Open-Meteo's answer for a place, or None when the answer is not one.
pub fn parse_reading(answer: &Value, place: &Place, units: &str) -> Option<Reading> {
    let now = answer.get("current")?;
    let temp = now.get("temperature_2m")?.as_f64()?;
    let is_day = now.get("is_day").and_then(Value::as_i64).unwrap_or(1) != 0;
    let (icon, label) = condition(now.get("weather_code").and_then(Value::as_u64).unwrap_or(3) as u32);
    let percent = |value: Option<f64>| value.map(|v| v.round().clamp(0.0, 100.0) as u8);
    let mut days = Vec::new();
    if let Some(daily) = answer.get("daily") {
        let dates = daily.get("time").and_then(Value::as_array).cloned().unwrap_or_default();
        for (index, date) in dates.iter().enumerate().take(DAYS) {
            let (day_icon, day_label) = condition(number(daily, "weather_code", index).map_or(3, |c| c as u32));
            days.push(Day {
                date: date.as_str().unwrap_or("").to_string(),
                icon: day_icon.into(),
                label: day_label.into(),
                high: number(daily, "temperature_2m_max", index),
                low: number(daily, "temperature_2m_min", index),
                rain: percent(number(daily, "precipitation_probability_max", index)),
            });
        }
    }
    Some(Reading {
        place: place.clone(),
        units: units_of(units).into(),
        temp,
        feels: now.get("apparent_temperature").and_then(Value::as_f64),
        humidity: percent(now.get("relative_humidity_2m").and_then(Value::as_f64)),
        wind: now.get("wind_speed_10m").and_then(Value::as_f64),
        wind_dir: now.get("wind_direction_10m").and_then(Value::as_f64).map(|d| (d.round() as i64).rem_euclid(360) as u16),
        is_day,
        icon: icon.into(),
        label: label.into(),
        days,
    })
}

fn client() -> Result<reqwest::blocking::Client, String> {
    reqwest::blocking::Client::builder()
        .user_agent(concat!("MemoryCoreCastle/", env!("CARGO_PKG_VERSION")))
        .timeout(Duration::from_secs(10))
        .build()
        .map_err(|_| "The weather service could not be reached.".to_string())
}

fn ask(url: &str, query: &[(&str, String)]) -> Result<Value, String> {
    let response = client()?.get(url).query(query).send().map_err(|_| "The weather service could not be reached. Check your internet connection.".to_string())?;
    if !response.status().is_success() {
        return Err(format!("The weather service answered with an error ({}).", response.status().as_u16()));
    }
    response.json::<Value>().map_err(|_| "The weather service sent something Castle could not read.".to_string())
}

/// Reads the weather at a place from Open-Meteo.
fn fetch(place: &Place, units: &str) -> Result<Reading, String> {
    let imperial = units_of(units) == "imperial";
    let answer = ask(
        FORECAST,
        &[
            ("latitude", place.lat.to_string()),
            ("longitude", place.lon.to_string()),
            ("current", "temperature_2m,relative_humidity_2m,apparent_temperature,is_day,weather_code,wind_speed_10m,wind_direction_10m".into()),
            ("daily", "weather_code,temperature_2m_max,temperature_2m_min,precipitation_probability_max".into()),
            ("timezone", "auto".into()),
            ("forecast_days", DAYS.to_string()),
            ("temperature_unit", if imperial { "fahrenheit" } else { "celsius" }.into()),
            ("wind_speed_unit", if imperial { "mph" } else { "kmh" }.into()),
        ],
    )?;
    parse_reading(&answer, place, units).ok_or_else(|| "The weather service had no weather for that place.".to_string())
}

/// The last reading and when it was taken.
static KEPT: Mutex<Option<(Instant, Reading)>> = Mutex::new(None);

fn kept_for(place: &Place, units: &str, now: Instant) -> Option<Reading> {
    let kept = KEPT.lock().ok()?;
    let (at, reading) = kept.as_ref()?;
    (now.duration_since(*at) < KEEP && reading.place == *place && reading.units == units_of(units)).then(|| reading.clone())
}

fn forget() {
    if let Ok(mut kept) = KEPT.lock() {
        *kept = None;
    }
}

/// The member's place and units.
#[tauri::command]
pub fn weather_get(store: tauri::State<'_, crate::settings::SettingsStore>) -> WeatherSettings {
    store.get().weather
}

/// Places that match a name the member typed.
#[tauri::command]
pub async fn weather_search(query: String) -> Result<Vec<Place>, String> {
    let name: String = query.trim().chars().filter(|c| !c.is_control()).take(80).collect();
    if name.chars().count() < 2 {
        return Ok(Vec::new());
    }
    tauri::async_runtime::spawn_blocking(move || {
        ask(SEARCH, &[("name", name), ("count", "6".into()), ("language", "en".into()), ("format", "json".into())]).map(|answer| parse_places(&answer))
    })
    .await
    .map_err(|_| "The search stopped before it finished.".to_string())?
}

/// Saves the place (or none, to forget it) and the units. The Settings page never overwrites them.
#[tauri::command]
pub fn weather_save(store: tauri::State<'_, crate::settings::SettingsStore>, place: Option<Place>, units: String) -> Result<WeatherSettings, String> {
    let place = match place {
        Some(place) => Some(clean_place(place).ok_or_else(|| "That place is not on the map.".to_string())?),
        None => None,
    };
    let mut next = store.get();
    next.weather = WeatherSettings { place, units: units_of(&units).into() };
    crate::settings::save(&store.path, &next).map_err(|_| "The weather settings could not be saved.".to_string())?;
    let saved = next.weather.clone();
    *store.current.lock().unwrap() = next;
    forget();
    Ok(saved)
}

/// The place for coordinates the computer gave (the member allowed Castle to ask where it is).
/// Open-Meteo has no street map, so the place is called "My location".
#[tauri::command]
pub fn weather_here(lat: f64, lon: f64) -> Result<Place, String> {
    let (lat, lon) = clean_coords(lat, lon).ok_or_else(|| "That location is not on the map.".to_string())?;
    Ok(Place { name: "My location".into(), region: String::new(), lat, lon })
}

/// The weather at the member's place, or None while no place is chosen. `force` skips the ten minutes.
#[tauri::command]
pub async fn weather_read(app: tauri::AppHandle, force: bool) -> Result<Option<Reading>, String> {
    use tauri::Manager;
    let settings = app.state::<crate::settings::SettingsStore>().get().weather;
    let Some(place) = settings.place else { return Ok(None) };
    if !force {
        if let Some(reading) = kept_for(&place, &settings.units, Instant::now()) {
            return Ok(Some(reading));
        }
    }
    let units = settings.units.clone();
    let reading = tauri::async_runtime::spawn_blocking(move || fetch(&place, &units)).await.map_err(|_| "The weather stopped before it finished.".to_string())??;
    if let Ok(mut kept) = KEPT.lock() {
        *kept = Some((Instant::now(), reading.clone()));
    }
    Ok(Some(reading))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn place() -> Place {
        Place { name: "Kuala Lumpur".into(), region: "Kuala Lumpur, Malaysia".into(), lat: 3.14, lon: 101.69 }
    }

    fn answer() -> Value {
        json!({
            "current": { "temperature_2m": 31.4, "relative_humidity_2m": 78, "apparent_temperature": 37.2, "is_day": 1, "weather_code": 95, "wind_speed_10m": 9.7, "wind_direction_10m": 215 },
            "daily": {
                "time": ["2026-11-18", "2026-11-19", "2026-11-20", "2026-11-21", "2026-11-22", "2026-11-23"],
                "weather_code": [95, 63, 2, 0, 3, 61],
                "temperature_2m_max": [33.1, 32.0, 33.5, 34.0, null, 31.0],
                "temperature_2m_min": [24.0, 23.5, 24.1, 24.6, 24.0, 23.0],
                "precipitation_probability_max": [80, 90, 20, 5, 30, 60]
            }
        })
    }

    #[test]
    fn weather_codes_become_an_icon_and_words() {
        assert_eq!(condition(0), ("clear", "Clear sky"));
        assert_eq!(condition(2), ("partly", "Partly cloudy"));
        assert_eq!(condition(63), ("rain", "Rain"));
        assert_eq!(condition(73), ("snow", "Snow"));
        assert_eq!(condition(96), ("storm", "Thunderstorm with hail"));
        assert_eq!(condition(45).0, "fog");
        assert_eq!(condition(999).0, "cloud", "an unknown code is shown as cloud, never as an error");
    }

    #[test]
    fn a_reading_is_read_from_the_answer_and_keeps_five_days() {
        let reading = parse_reading(&answer(), &place(), "metric").unwrap();
        assert_eq!(reading.temp, 31.4);
        assert_eq!(reading.feels, Some(37.2));
        assert_eq!(reading.humidity, Some(78));
        assert_eq!(reading.wind, Some(9.7));
        assert_eq!(reading.wind_dir, Some(215));
        assert!(reading.is_day);
        assert_eq!((reading.icon.as_str(), reading.label.as_str()), ("storm", "Thunderstorm"));
        assert_eq!(reading.days.len(), 5, "the sixth day is left out");
        assert_eq!(reading.days[1].icon, "rain");
        assert_eq!((reading.days[0].high, reading.days[0].low, reading.days[0].rain), (Some(33.1), Some(24.0), Some(80)));
        assert_eq!(reading.days[4].high, None, "a missing figure stays unknown instead of becoming 0");
    }

    #[test]
    fn what_the_service_leaves_out_stays_unknown_and_nonsense_is_refused() {
        let thin = json!({ "current": { "temperature_2m": -3.0 } });
        let reading = parse_reading(&thin, &place(), "imperial").unwrap();
        assert_eq!((reading.feels, reading.humidity, reading.wind, reading.wind_dir), (None, None, None, None));
        assert!(reading.days.is_empty());
        assert_eq!(reading.units, "imperial");
        assert!(parse_reading(&json!({ "error": true, "reason": "nope" }), &place(), "metric").is_none());
        assert!(parse_reading(&json!({ "current": {} }), &place(), "metric").is_none());
    }

    #[test]
    fn a_wind_direction_stays_on_the_compass() {
        let mut a = answer();
        a["current"]["wind_direction_10m"] = json!(360.0);
        assert_eq!(parse_reading(&a, &place(), "metric").unwrap().wind_dir, Some(0));
    }

    #[test]
    fn places_are_read_from_a_search_with_their_region() {
        let found = json!({ "results": [
            { "name": "Springfield", "admin1": "Illinois", "country": "United States", "latitude": 39.80, "longitude": -89.64 },
            { "name": "Singapore", "admin1": "Singapore", "country": "Singapore", "latitude": 1.2897, "longitude": 103.8501 },
            { "name": "Broken", "latitude": 1.0 }
        ] });
        let places = parse_places(&found);
        assert_eq!(places.len(), 2, "an entry without coordinates is skipped");
        assert_eq!(places[0].region, "Illinois, United States");
        assert_eq!(places[1].region, "", "a region or country that only repeats the name is not shown twice");
        assert_eq!((places[1].lat, places[1].lon), (1.29, 103.85), "coordinates are kept to two decimals");
        assert!(parse_places(&json!({})).is_empty(), "a search with no results is an empty list");
    }

    #[test]
    fn only_places_on_the_map_are_kept() {
        assert_eq!(clean_coords(3.1412, 101.6869), Some((3.14, 101.69)));
        assert_eq!(clean_coords(91.0, 0.0), None);
        assert_eq!(clean_coords(0.0, 181.0), None);
        assert_eq!(clean_coords(f64::NAN, 0.0), None);
        assert!(clean_place(Place { name: "   ".into(), region: String::new(), lat: 1.0, lon: 1.0 }).is_none());
        let long = clean_place(Place { name: format!("  {}  ", "x".repeat(200)), region: "r\n".into(), lat: 1.234, lon: 2.0 }).unwrap();
        assert_eq!(long.name.chars().count(), 60);
        assert_eq!(long.region, "r");
    }

    #[test]
    fn the_units_are_metric_unless_imperial_is_asked() {
        assert_eq!(units_of("imperial"), "imperial");
        assert_eq!(units_of(" Imperial "), "imperial");
        assert_eq!(units_of("kelvin"), "metric");
        assert_eq!(units_of(""), "metric");
        assert_eq!(WeatherSettings::default().units, "metric");
    }

    #[test]
    fn an_older_settings_file_without_weather_reads_as_no_place() {
        let read: WeatherSettings = serde_json::from_str("{}").unwrap();
        assert_eq!(read, WeatherSettings::default());
        let read: WeatherSettings = serde_json::from_str(r#"{"place":{"name":"Ipoh","lat":4.6,"lon":101.08},"units":"imperial"}"#).unwrap();
        assert_eq!(read.place.unwrap().name, "Ipoh");
        assert_eq!(read.units, "imperial");
    }

    #[test]
    fn a_reading_is_reused_for_ten_minutes_for_the_same_place_and_units_only() {
        forget();
        let reading = parse_reading(&answer(), &place(), "metric").unwrap();
        *KEPT.lock().unwrap() = Some((Instant::now(), reading.clone()));
        let soon = Instant::now() + Duration::from_secs(5);
        assert_eq!(kept_for(&place(), "metric", soon), Some(reading.clone()));
        assert_eq!(kept_for(&place(), "imperial", soon), None, "other units are asked again");
        let mut elsewhere = place();
        elsewhere.lat = 4.0;
        assert_eq!(kept_for(&elsewhere, "metric", soon), None, "another place is asked again");
        assert_eq!(kept_for(&place(), "metric", Instant::now() + Duration::from_secs(11 * 60)), None, "after ten minutes it is asked again");
        forget();
        assert_eq!(kept_for(&place(), "metric", soon), None);
    }
}
