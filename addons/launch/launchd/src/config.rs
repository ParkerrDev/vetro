//! The settings, from the TOML file the addon links into ~/.config/vetro:
//! where launches come from and how often to look, and the launch and weather
//! cards' own sections, which are handed on to them as they are.

use serde_json::{Map, Value, json};
use toml::Table;

/// Launch Library 2's id for `SpaceX`.
const SPACEX: u32 = 121;
const MAX_PROVIDERS: usize = 16;

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum Error {
    #[error("the settings are not TOML: {0}")]
    Toml(String),
    #[error("launches.{0} is not a number from {1} to {2}")]
    Range(&'static str, i64, i64),
    #[error("launches.api must be an https:// address")]
    Api,
    #[error("launches.providers must be a list of Launch Library provider ids")]
    Providers,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Settings {
    pub providers: Vec<u32>,
    pub upcoming_count: i64,
    pub refresh_minutes: i64,
    pub refresh_minutes_near_launch: i64,
    pub near_launch_minutes: i64,
    pub api: String,
    /// The launch card's section, over its defaults.
    pub overlay: Value,
    /// The weather card's section, over its defaults.
    pub weather: Value,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            providers: vec![SPACEX],
            upcoming_count: 8,
            refresh_minutes: 15,
            refresh_minutes_near_launch: 5,
            near_launch_minutes: 120,
            api: "https://ll.thespacedevs.com/2.3.0".to_owned(),
            overlay: default_overlay(),
            weather: default_weather(),
        }
    }
}

fn default_overlay() -> Value {
    json!({
        "enabled": true, "outputs": ["*"], "exclude_outputs": ["HEADLESS-*"],
        "corner": "top-left", "upcoming_rows": 3, "live_scale": 0.86,
    })
}

fn default_weather() -> Value {
    json!({
        "enabled": true, "outputs": ["*"], "exclude_outputs": ["HEADLESS-*"],
        "corner": "top-right", "hours": 6, "hide_when_live": true,
        "location": "", "units": "fahrenheit",
    })
}

fn bounded(
    table: &Table,
    key: &'static str,
    low: i64,
    high: i64,
    fallback: i64,
) -> Result<i64, Error> {
    match table.get(key) {
        None => Ok(fallback),
        Some(value) => value
            .as_integer()
            .filter(|n| (low..=high).contains(n))
            .ok_or(Error::Range(key, low, high)),
    }
}

fn providers(table: &Table) -> Result<Vec<u32>, Error> {
    let Some(value) = table.get("providers") else {
        return Ok(vec![SPACEX]);
    };
    let list = value
        .as_array()
        .filter(|list| !list.is_empty() && list.len() <= MAX_PROVIDERS)
        .ok_or(Error::Providers)?;
    list.iter()
        .map(|id| {
            id.as_integer()
                .and_then(|n| u32::try_from(n).ok())
                .ok_or(Error::Providers)
        })
        .collect()
}

/// A card's section laid over its defaults, one level deep, as the cards
/// read it: a key the file sets wins, the rest keep their default.
fn section(file: &Table, name: &str, defaults: Value) -> Value {
    let mut merged: Map<String, Value> = match defaults {
        Value::Object(map) => map,
        _ => Map::new(),
    };
    if let Some(toml::Value::Table(table)) = file.get(name) {
        for (key, value) in table {
            if let Ok(converted) = serde_json::to_value(value) {
                merged.insert(key.clone(), converted);
            }
        }
    }
    Value::Object(merged)
}

/// The settings in a file's text. Sections other than [launches], [overlay]
/// and [weather] (those of the video wallpaper, which is gone) are ignored.
///
/// # Errors
/// When the text is not TOML or a [launches] value is out of its range.
pub fn parse(text: &str) -> Result<Settings, Error> {
    let file: Table = text
        .parse()
        .map_err(|error: toml::de::Error| Error::Toml(error.message().to_owned()))?;
    let base = Settings::default();
    let empty = Table::new();
    let launches = file
        .get("launches")
        .and_then(toml::Value::as_table)
        .unwrap_or(&empty);
    let api = match launches.get("api") {
        None => base.api,
        Some(value) => value
            .as_str()
            .filter(|api| api.starts_with("https://"))
            .ok_or(Error::Api)?
            .to_owned(),
    };
    Ok(Settings {
        providers: providers(launches)?,
        upcoming_count: bounded(launches, "upcoming_count", 1, 50, base.upcoming_count)?,
        refresh_minutes: bounded(launches, "refresh_minutes", 1, 1440, base.refresh_minutes)?,
        refresh_minutes_near_launch: bounded(
            launches,
            "refresh_minutes_near_launch",
            1,
            1440,
            base.refresh_minutes_near_launch,
        )?,
        near_launch_minutes: bounded(
            launches,
            "near_launch_minutes",
            0,
            10_080,
            base.near_launch_minutes,
        )?,
        api,
        overlay: section(&file, "overlay", base.overlay),
        weather: section(&file, "weather", base.weather),
    })
}

#[cfg(test)]
mod tests {
    fn at(value: &serde_json::Value, pointer: &str) -> serde_json::Value {
        value
            .pointer(pointer)
            .cloned()
            .unwrap_or(serde_json::Value::Null)
    }

    use super::{Error, Settings, parse};

    #[test]
    fn an_empty_file_is_the_defaults() {
        assert_eq!(parse(""), Ok(Settings::default()));
    }

    #[test]
    fn reads_the_launch_settings_and_ignores_the_wallpaper_ones() {
        let text =
            "[launches]\nupcoming_count = 4\nproviders = [121, 30]\n[wallpaper]\nenabled = true\n";
        let settings = parse(text).unwrap_or_default();
        assert_eq!(settings.upcoming_count, 4);
        assert_eq!(settings.providers, vec![121, 30]);
    }

    #[test]
    fn hands_the_cards_their_sections_over_the_defaults() {
        let settings = parse("[overlay]\ncorner = \"bottom-left\"\n").unwrap_or_default();
        assert_eq!(at(&settings.overlay, "/corner"), "bottom-left");
        assert_eq!(at(&settings.overlay, "/upcoming_rows"), 3);
        assert_eq!(at(&settings.weather, "/corner"), "top-right");
    }

    #[test]
    fn refuses_values_out_of_range() {
        assert_eq!(
            parse("[launches]\nrefresh_minutes = 0\n"),
            Err(Error::Range("refresh_minutes", 1, 1440))
        );
        assert_eq!(
            parse("[launches]\napi = \"http://example.org\"\n"),
            Err(Error::Api)
        );
        assert_eq!(parse("[launches]\nproviders = []\n"), Err(Error::Providers));
        assert!(matches!(parse("[launches"), Err(Error::Toml(_))));
    }
}
