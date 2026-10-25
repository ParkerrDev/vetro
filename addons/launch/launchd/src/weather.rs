//! The weather card's forecast, from Open-Meteo (free, with no key): where
//! the forecast is for, the addresses to ask, and the answers made into the
//! file the card reads. The binary does the asking.
//!
//! The place is the `location` of the [weather] settings ("Fresno CA",
//! "Paris", "London, UK"), found by Open-Meteo's geocoding, or with none set,
//! where this machine's address says it is (behind a VPN, the VPN's city).

use crate::clock;
use serde_json::{Value, json};

/// How many hours of forecast the file holds, from the current one on.
const HOURS: usize = 12;
const MAX_DAYS: usize = 7;
const MAX_HOURS_SCANNED: usize = 400;
const MAX_RESULTS: usize = 20;
const MAX_LOCATION: usize = 120;

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum Error {
    #[error("the forecast has no {0}")]
    Missing(&'static str),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Units {
    #[default]
    Fahrenheit,
    Celsius,
}

impl Units {
    /// "celsius" (or "metric") is Celsius; anything else Fahrenheit.
    #[must_use]
    pub fn named(name: &str) -> Self {
        match name.trim().to_ascii_lowercase().as_str() {
            "celsius" | "metric" | "c" => Self::Celsius,
            _ => Self::Fahrenheit,
        }
    }

    fn degrees(self) -> &'static str {
        match self {
            Self::Fahrenheit => "°F",
            Self::Celsius => "°C",
        }
    }

    fn wind(self) -> &'static str {
        match self {
            Self::Fahrenheit => "mph",
            Self::Celsius => "km/h",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Place {
    pub lat: f64,
    pub lon: f64,
    pub label: String,
}

/// A WMO weather code in words, and as a Material Symbols icon.
#[must_use]
pub fn describe(code: i64, day: bool) -> (&'static str, &'static str) {
    let clear = if day { "sunny" } else { "clear_night" };
    match code {
        0 => ("Clear", clear),
        1 => ("Mostly clear", clear),
        2 if day => ("Partly cloudy", "partly_cloudy_day"),
        2 => ("Partly cloudy", "partly_cloudy_night"),
        3 => ("Overcast", "cloud"),
        45 | 48 => ("Fog", "foggy"),
        51..=57 => ("Drizzle", "rainy_light"),
        61..=64 | 66 | 67 => ("Rain", "rainy"),
        65 => ("Heavy rain", "rainy_heavy"),
        71..=77 => ("Snow", "weather_snowy"),
        80..=82 => ("Showers", "rainy"),
        85 | 86 => ("Snow showers", "weather_snowy"),
        95 => ("Thunderstorm", "thunderstorm"),
        96 | 99 => ("Thunderstorm, hail", "weather_hail"),
        _ => ("Unknown", "question_mark"),
    }
}

/// The states of the US, for "Fresno CA" and for labels like "Fresno, CA".
const STATES: [(&str, &str); 51] = [
    ("AL", "Alabama"),
    ("AK", "Alaska"),
    ("AZ", "Arizona"),
    ("AR", "Arkansas"),
    ("CA", "California"),
    ("CO", "Colorado"),
    ("CT", "Connecticut"),
    ("DE", "Delaware"),
    ("DC", "District of Columbia"),
    ("FL", "Florida"),
    ("GA", "Georgia"),
    ("HI", "Hawaii"),
    ("ID", "Idaho"),
    ("IL", "Illinois"),
    ("IN", "Indiana"),
    ("IA", "Iowa"),
    ("KS", "Kansas"),
    ("KY", "Kentucky"),
    ("LA", "Louisiana"),
    ("ME", "Maine"),
    ("MD", "Maryland"),
    ("MA", "Massachusetts"),
    ("MI", "Michigan"),
    ("MN", "Minnesota"),
    ("MS", "Mississippi"),
    ("MO", "Missouri"),
    ("MT", "Montana"),
    ("NE", "Nebraska"),
    ("NV", "Nevada"),
    ("NH", "New Hampshire"),
    ("NJ", "New Jersey"),
    ("NM", "New Mexico"),
    ("NY", "New York"),
    ("NC", "North Carolina"),
    ("ND", "North Dakota"),
    ("OH", "Ohio"),
    ("OK", "Oklahoma"),
    ("OR", "Oregon"),
    ("PA", "Pennsylvania"),
    ("RI", "Rhode Island"),
    ("SC", "South Carolina"),
    ("SD", "South Dakota"),
    ("TN", "Tennessee"),
    ("TX", "Texas"),
    ("UT", "Utah"),
    ("VT", "Vermont"),
    ("VA", "Virginia"),
    ("WA", "Washington"),
    ("WV", "West Virginia"),
    ("WI", "Wisconsin"),
    ("WY", "Wyoming"),
];

fn state_code(name: &str) -> Option<&'static str> {
    STATES
        .iter()
        .find(|(_, full)| *full == name)
        .map(|(code, _)| *code)
}

fn state_name(code: &str) -> Option<&'static str> {
    STATES
        .iter()
        .find(|(short, _)| short.eq_ignore_ascii_case(code))
        .map(|(_, full)| *full)
}

/// "Fresno" and where it is: the state's code in the US, else the country's.
fn label(city: &str, country: &str, region: &str) -> String {
    let city = if city.is_empty() { "Here" } else { city };
    let part = if country == "US" {
        state_code(region).unwrap_or("")
    } else {
        country
    };
    if part.is_empty() {
        city.to_owned()
    } else {
        format!("{city}, {part}")
    }
}

/// The names to look a location up by, each with a hint (a state or a
/// country) that picks among places of the same name, in the order to try.
/// "Fresno, CA" is Fresno in CA; "Fresno CA" too, as a short last word is
/// almost surely a state or a country; "New York" is tried whole first.
#[must_use]
pub fn attempts(location: &str) -> Vec<(String, String)> {
    let text: String = location.trim().chars().take(MAX_LOCATION).collect();
    if let Some((name, hint)) = text.rsplit_once(',') {
        return vec![(name.trim().to_owned(), hint.trim().to_owned())];
    }
    let whole = (text.clone(), String::new());
    match text.rsplit_once(' ') {
        Some((name, hint)) if hint.chars().count() <= 3 => {
            vec![(name.trim().to_owned(), hint.to_owned()), whole]
        }
        Some((name, hint)) => vec![whole, (name.trim().to_owned(), hint.to_owned())],
        None => vec![whole],
    }
}

/// Text made safe for a URL's query.
fn encode(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for byte in text.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~') {
            out.push(char::from(byte));
        } else {
            out.push('%');
            for nibble in [byte >> 4, byte & 0x0f] {
                out.push(
                    char::from_digit(u32::from(nibble), 16)
                        .map_or('0', |digit| digit.to_ascii_uppercase()),
                );
            }
        }
    }
    out
}

/// Where to look a name up.
#[must_use]
pub fn geocode_url(name: &str) -> String {
    format!(
        "https://geocoding-api.open-meteo.com/v1/search?count={MAX_RESULTS}&language=en&format=json&name={}",
        encode(name)
    )
}

fn text<'a>(value: &'a Value, key: &str) -> &'a str {
    value.get(key).and_then(Value::as_str).unwrap_or("")
}

fn hint_fits(result: &Value, hint: &str) -> bool {
    if hint.is_empty() {
        return true;
    }
    let hint = hint.to_lowercase();
    let code = text(result, "country_code").to_lowercase();
    let state = text(result, "admin1");
    hint == code
        || (hint == "uk" && code == "gb")
        || hint == text(result, "country").to_lowercase()
        || hint == state.to_lowercase()
        || (code == "us" && state_name(&hint) == Some(state))
}

/// The place a geocoding answer gives for a name, the first that fits the hint.
#[must_use]
pub fn pick(answer: &Value, hint: &str) -> Option<Place> {
    let results = answer.get("results").and_then(Value::as_array)?;
    let best = results
        .iter()
        .take(MAX_RESULTS)
        .find(|result| hint_fits(result, hint))?;
    Some(Place {
        lat: best.get("latitude").and_then(Value::as_f64)?,
        lon: best.get("longitude").and_then(Value::as_f64)?,
        label: label(
            text(best, "name"),
            text(best, "country_code"),
            text(best, "admin1"),
        ),
    })
}

/// Where geojs.io says this machine is (asked first: ipinfo.io limits shared
/// VPN addresses).
pub const GEOJS: &str = "https://get.geojs.io/v1/ip/geo.json";
/// Where ipinfo.io says it is, when geojs.io does not answer.
pub const IPINFO: &str = "https://ipinfo.io/json";

fn number(value: &Value, key: &str) -> Option<f64> {
    let field = value.get(key)?;
    field
        .as_f64()
        .or_else(|| field.as_str().and_then(|text| text.trim().parse().ok()))
}

/// The place in geojs.io's answer.
#[must_use]
pub fn from_geojs(answer: &Value) -> Option<Place> {
    Some(Place {
        lat: number(answer, "latitude")?,
        lon: number(answer, "longitude")?,
        label: label(
            text(answer, "city"),
            text(answer, "country_code"),
            text(answer, "region"),
        ),
    })
}

/// The place in ipinfo.io's answer ("loc": "36.7,-119.8").
#[must_use]
pub fn from_ipinfo(answer: &Value) -> Option<Place> {
    let (lat, lon) = text(answer, "loc").split_once(',')?;
    Some(Place {
        lat: lat.trim().parse().ok()?,
        lon: lon.trim().parse().ok()?,
        label: label(
            text(answer, "city"),
            text(answer, "country"),
            text(answer, "region"),
        ),
    })
}

/// Where to ask for a place's forecast.
#[must_use]
pub fn forecast_url(place: &Place, units: Units) -> String {
    let (temperature, wind) = match units {
        Units::Fahrenheit => ("fahrenheit", "mph"),
        Units::Celsius => ("celsius", "kmh"),
    };
    format!(
        "https://api.open-meteo.com/v1/forecast?latitude={}&longitude={}\
         &current=temperature_2m,apparent_temperature,weather_code,is_day,wind_speed_10m,relative_humidity_2m\
         &hourly=temperature_2m,weather_code,precipitation_probability,is_day\
         &daily=weather_code,temperature_2m_max,temperature_2m_min,sunrise,sunset,precipitation_probability_max\
         &temperature_unit={temperature}&wind_speed_unit={wind}&timezone=auto&forecast_days=4&timeformat=unixtime",
        place.lat, place.lon
    )
}

/// A whole number from a reading, or null where there is none.
fn whole(value: Option<&Value>) -> Value {
    value
        .and_then(Value::as_f64)
        .and_then(|reading| clock::whole_seconds(reading.round()))
        .map_or(Value::Null, Value::from)
}

fn code(value: Option<&Value>) -> i64 {
    value.and_then(Value::as_i64).unwrap_or(-1)
}

fn flag(value: Option<&Value>) -> bool {
    value.and_then(Value::as_i64).is_some_and(|n| n != 0)
}

fn series<'a>(table: &'a Value, key: &'static str) -> Result<&'a [Value], Error> {
    table
        .get(key)
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .ok_or(Error::Missing(key))
}

fn hourly(answer: &Value, now: i64) -> Result<Vec<Value>, Error> {
    let table = answer.get("hourly").ok_or(Error::Missing("hourly"))?;
    let times = series(table, "time")?;
    let (temps, codes) = (
        series(table, "temperature_2m")?,
        series(table, "weather_code")?,
    );
    let (rain, days) = (
        series(table, "precipitation_probability")?,
        series(table, "is_day")?,
    );
    let mut hours = Vec::with_capacity(HOURS);
    for (index, time) in times.iter().enumerate().take(MAX_HOURS_SCANNED) {
        let Some(start) = time.as_i64() else { continue };
        // from the start of the current hour on
        if start.saturating_add(3600) <= now {
            continue;
        }
        let day = flag(days.get(index));
        hours.push(json!({
            "time": start,
            "temp": whole(temps.get(index)),
            "precip": whole(rain.get(index)),
            "isDay": day,
            "icon": describe(code(codes.get(index)), day).1,
        }));
        if hours.len() >= HOURS {
            break;
        }
    }
    Ok(hours)
}

fn daily(answer: &Value) -> Result<Vec<Value>, Error> {
    let table = answer.get("daily").ok_or(Error::Missing("daily"))?;
    let times = series(table, "time")?;
    let (codes, highs, lows) = (
        series(table, "weather_code")?,
        series(table, "temperature_2m_max")?,
        series(table, "temperature_2m_min")?,
    );
    let (rises, sets, rain) = (
        series(table, "sunrise")?,
        series(table, "sunset")?,
        series(table, "precipitation_probability_max")?,
    );
    Ok(times
        .iter()
        .enumerate()
        .take(MAX_DAYS)
        .map(|(index, time)| {
            json!({
                "time": time,
                "high": whole(highs.get(index)),
                "low": whole(lows.get(index)),
                "precip": whole(rain.get(index)),
                "sunrise": rises.get(index).cloned().unwrap_or(Value::Null),
                "sunset": sets.get(index).cloned().unwrap_or(Value::Null),
                "icon": describe(code(codes.get(index)), true).1,
            })
        })
        .collect())
}

/// The file the card reads, from a forecast for a place.
///
/// # Errors
/// When the answer has no current reading, or no hourly or daily table.
pub fn forecast(answer: &Value, place: &Place, units: Units, now: i64) -> Result<Value, Error> {
    let current = answer.get("current").ok_or(Error::Missing("current"))?;
    let day = flag(current.get("is_day"));
    let (words, icon) = describe(code(current.get("weather_code")), day);
    let days = daily(answer)?;
    Ok(json!({
        "ready": true,
        "error": "",
        "updated": now,
        "location": place.label,
        "unit": units.degrees(),
        "windUnit": units.wind(),
        "current": {
            "temp": whole(current.get("temperature_2m")),
            "feels": whole(current.get("apparent_temperature")),
            "code": code(current.get("weather_code")),
            "isDay": day,
            "wind": whole(current.get("wind_speed_10m")),
            "humidity": whole(current.get("relative_humidity_2m")),
            "text": words,
            "icon": icon,
        },
        "today": days.first().cloned().unwrap_or_else(|| json!({})),
        "hourly": hourly(answer, now)?,
        "daily": days,
    }))
}

/// The file after a failed look: the last forecast kept, with why it is old.
#[must_use]
pub fn failed(last: Option<&Value>, why: &str) -> Value {
    let mut file = last.cloned().unwrap_or_else(|| json!({ "ready": false }));
    if let Some(fields) = file.as_object_mut() {
        fields.insert("error".to_owned(), json!(why));
    }
    file
}

#[cfg(test)]
mod tests {
    use super::{Place, Units, attempts, describe, encode, forecast, from_ipinfo, pick};
    use serde_json::json;

    #[test]
    fn splits_a_location_into_a_name_and_a_hint() {
        let pair = |a: &str, b: &str| (a.to_owned(), b.to_owned());
        assert_eq!(
            attempts("Fresno CA"),
            vec![pair("Fresno", "CA"), pair("Fresno CA", "")]
        );
        assert_eq!(attempts("London, UK"), vec![pair("London", "UK")]);
        assert_eq!(
            attempts("New York"),
            vec![pair("New York", ""), pair("New", "York")]
        );
        assert_eq!(attempts(" Paris "), vec![pair("Paris", "")]);
    }

    #[test]
    fn encodes_a_name_for_the_address() {
        assert_eq!(encode("Fresno CA"), "Fresno%20CA");
        assert_eq!(encode("São Paulo"), "S%C3%A3o%20Paulo");
    }

    #[test]
    fn picks_the_place_the_hint_names() {
        let answer = json!({ "results": [
            { "name": "Fresno", "latitude": 29.5, "longitude": -95.4, "country_code": "US", "admin1": "Texas" },
            { "name": "Fresno", "latitude": 36.7, "longitude": -119.8, "country_code": "US", "admin1": "California" },
        ]});
        let place = pick(&answer, "CA");
        assert_eq!(place.map(|p| p.label), Some("Fresno, CA".to_owned()));
        assert!(pick(&answer, "ZZ").is_none());
        assert_eq!(
            pick(&answer, "").map(|p| p.label),
            Some("Fresno, TX".to_owned())
        );
    }

    #[test]
    fn reads_where_ipinfo_says_it_is() {
        let answer = json!({ "loc": "48.85,2.35", "city": "Paris", "country": "FR", "region": "Île-de-France" });
        let place = from_ipinfo(&answer);
        assert_eq!(place.map(|p| p.label), Some("Paris, FR".to_owned()));
    }

    #[test]
    fn names_the_weather() {
        assert_eq!(describe(0, false), ("Clear", "clear_night"));
        assert_eq!(describe(65, true), ("Heavy rain", "rainy_heavy"));
        assert_eq!(describe(1000, true).0, "Unknown");
    }

    #[test]
    fn makes_the_file_from_a_forecast() {
        let answer = json!({
            "current": { "temperature_2m": 71.6, "apparent_temperature": 70.2, "weather_code": 2, "is_day": 1, "wind_speed_10m": 4.4, "relative_humidity_2m": 40 },
            "hourly": { "time": [0, 3600, 7200], "temperature_2m": [60.0, 61.4, 62.5], "weather_code": [0, 0, 3], "precipitation_probability": [0, 10, 20], "is_day": [0, 1, 1] },
            "daily": { "time": [0], "weather_code": [61], "temperature_2m_max": [80.4], "temperature_2m_min": [50.5], "sunrise": [100], "sunset": [200], "precipitation_probability_max": [30] },
        });
        let place = Place {
            lat: 1.0,
            lon: 2.0,
            label: "Fresno, CA".to_owned(),
        };
        let file = forecast(&answer, &place, Units::Fahrenheit, 4000).unwrap_or_default();
        assert_eq!(file.pointer("/current/temp"), Some(&json!(72)));
        assert_eq!(
            file.pointer("/current/icon"),
            Some(&json!("partly_cloudy_day"))
        );
        assert_eq!(file.pointer("/hourly/0/time"), Some(&json!(3600)));
        assert_eq!(file.pointer("/today/high"), Some(&json!(80)));
        assert_eq!(file.pointer("/unit"), Some(&json!("°F")));
        assert!(forecast(&json!({}), &place, Units::Celsius, 0).is_err());
    }
}
