//! A launch as Launch Library 2 gives it, in the shape the launch card, the
//! lock screen and the login screen read (the one the Python daemon wrote,
//! field for field).

use crate::clock;
use serde_json::{Value, json};

const MAX_VIDEOS: usize = 32;
const ROCKET_SHORT: [(&str, &str); 3] = [
    ("Falcon Heavy", "FH"),
    ("Falcon 9", "F9"),
    ("Starship", "SS"),
];
const PAD_SHORT: [(&str, &str); 2] = [("Space Launch Complex", "SLC-"), ("Launch Complex", "LC-")];

/// The text at a path of keys, or "" where there is none.
fn text<'a>(value: &'a Value, path: &[&str]) -> &'a str {
    path.iter()
        .try_fold(value, |at, key| at.get(key))
        .and_then(Value::as_str)
        .unwrap_or("")
}

fn time(value: &Value, key: &str) -> Value {
    value
        .get(key)
        .and_then(Value::as_str)
        .and_then(clock::parse)
        .map_or(Value::Null, Value::from)
}

/// "F9" for "Falcon 9 Block 5"; the first three letters, in capitals, for a
/// rocket not in the list.
#[must_use]
pub fn rocket_short(rocket: &str) -> String {
    ROCKET_SHORT
        .iter()
        .find(|(prefix, _)| rocket.starts_with(prefix))
        .map_or_else(
            || rocket.chars().take(3).collect::<String>().to_uppercase(),
            |(_, short)| (*short).to_owned(),
        )
}

/// "SLC-40" for "Space Launch Complex 40".
#[must_use]
pub fn pad_short(pad: &str) -> String {
    for (long, short) in PAD_SHORT {
        if let Some(rest) = pad.strip_prefix(long) {
            if rest.starts_with(char::is_whitespace) {
                return format!("{short}{}", rest.trim_start());
            }
        }
    }
    pad.to_owned()
}

/// "Cape Canaveral SFS, FL" for "Cape Canaveral SFS, FL, USA".
#[must_use]
pub fn location_short(location: &str) -> String {
    for country in [", USA", ", United States of America"] {
        if let Some(rest) = location.strip_suffix(country) {
            return rest.trim_end().to_owned();
        }
    }
    location.to_owned()
}

/// "Bandwagon 5" for "Falcon 9 Block 5 | Bandwagon 5"; the mission's name, or
/// the launch's, when the name has no bar.
fn title(raw: &Value) -> String {
    let name = text(raw, &["name"]);
    match name.split_once(" | ") {
        Some((_, title)) => title.to_owned(),
        None if !text(raw, &["mission", "name"]).is_empty() => {
            text(raw, &["mission", "name"]).to_owned()
        }
        None => name.to_owned(),
    }
}

fn video(raw: &Value) -> Value {
    json!({
        "url": raw.get("url").cloned().unwrap_or(Value::Null),
        "publisher": text(raw, &["publisher"]),
        "source": text(raw, &["source"]),
        "type": text(raw, &["type", "name"]),
        "priority": raw.get("priority").and_then(Value::as_i64).unwrap_or(99),
        "live": raw.get("live").and_then(Value::as_bool).unwrap_or(false),
        "title": text(raw, &["title"]),
        "start": time(raw, "start_time"),
        "end": time(raw, "end_time"),
    })
}

fn videos(raw: &Value) -> Value {
    let list = raw
        .get("vid_urls")
        .and_then(Value::as_array)
        .map_or(&[][..], Vec::as_slice);
    Value::Array(list.iter().take(MAX_VIDEOS).map(video).collect())
}

/// One launch, normalised.
#[must_use]
pub fn normalize(raw: &Value) -> Value {
    let rocket = match text(raw, &["rocket", "configuration", "full_name"]) {
        "" => text(raw, &["rocket", "configuration", "name"]),
        full => full,
    };
    let pad = text(raw, &["pad", "name"]);
    let location = text(raw, &["pad", "location", "name"]);
    json!({
        "id": raw.get("id").cloned().unwrap_or(Value::Null),
        "slug": text(raw, &["slug"]),
        "name": text(raw, &["name"]),
        "title": title(raw),
        "rocket": rocket,
        "rocket_short": rocket_short(rocket),
        "mission": text(raw, &["mission", "name"]),
        "mission_type": text(raw, &["mission", "type"]),
        "orbit": text(raw, &["mission", "orbit", "abbrev"]),
        "description": text(raw, &["mission", "description"]),
        "pad": pad,
        "pad_short": pad_short(pad),
        "location": location,
        "location_short": location_short(location),
        "net": raw.get("net").cloned().unwrap_or(Value::Null),
        "net_epoch": time(raw, "net"),
        "net_precision": text(raw, &["net_precision", "abbrev"]),
        "window_start": time(raw, "window_start"),
        "window_end": time(raw, "window_end"),
        "status": text(raw, &["status", "abbrev"]),
        "status_name": text(raw, &["status", "name"]),
        "probability": raw.get("probability").cloned().unwrap_or(Value::Null),
        "webcast_live": raw.get("webcast_live").and_then(Value::as_bool).unwrap_or(false),
        "image": text(raw, &["image", "image_url"]),
        "thumbnail": text(raw, &["image", "thumbnail_url"]),
        "vid_urls": videos(raw),
    })
}

/// When a launch lifts off, in seconds since the epoch, as normalised (a
/// whole number here, a float in a file the Python daemon wrote).
#[must_use]
pub fn net(launch: &Value) -> Option<i64> {
    let value = launch.get("net_epoch")?;
    value
        .as_i64()
        .or_else(|| value.as_f64().and_then(clock::whole_seconds))
}

#[cfg(test)]
mod tests {
    fn at(value: &serde_json::Value, pointer: &str) -> serde_json::Value {
        value
            .pointer(pointer)
            .cloned()
            .unwrap_or(serde_json::Value::Null)
    }

    use super::{location_short, normalize, pad_short, rocket_short};
    use serde_json::json;

    #[test]
    fn shortens_as_the_card_does() {
        assert_eq!(rocket_short("Falcon 9 Block 5"), "F9");
        assert_eq!(rocket_short("Electron"), "ELE");
        assert_eq!(pad_short("Space Launch Complex 40"), "SLC-40");
        assert_eq!(pad_short("Launch Complex 39A"), "LC-39A");
        assert_eq!(pad_short("Launch Complexity"), "Launch Complexity");
        assert_eq!(
            location_short("Cape Canaveral SFS, FL, USA"),
            "Cape Canaveral SFS, FL"
        );
    }

    #[test]
    fn normalises_a_launch() {
        let raw = json!({
            "id": "abc", "name": "Falcon 9 Block 5 | Bandwagon 5", "net": "2026-10-19T10:30:00Z",
            "rocket": {"configuration": {"full_name": "Falcon 9 Block 5"}},
            "pad": {"name": "Space Launch Complex 40", "location": {"name": "Cape Canaveral SFS, FL, USA"}},
            "status": {"abbrev": "Go", "name": "Go for Launch"}, "probability": 55,
            "vid_urls": [{"url": "https://example.org/v", "publisher": "SpaceX", "priority": 10}],
        });
        let launch = normalize(&raw);
        assert_eq!(at(&launch, "/title"), "Bandwagon 5");
        assert_eq!(at(&launch, "/rocket_short"), "F9");
        assert_eq!(at(&launch, "/pad_short"), "SLC-40");
        assert_eq!(at(&launch, "/net_epoch"), 1_792_405_800);
        assert_eq!(at(&launch, "/status"), "Go");
        assert_eq!(at(&launch, "/vid_urls/0/publisher"), "SpaceX");
        assert_eq!(at(&launch, "/vid_urls/0/live"), false);
    }

    #[test]
    fn reads_the_lift_off_time_as_either_kind_of_number() {
        assert_eq!(
            super::net(&json!({"net_epoch": 1_792_405_800})),
            Some(1_792_405_800)
        );
        assert_eq!(
            super::net(&json!({"net_epoch": 1_792_405_800.0})),
            Some(1_792_405_800)
        );
        assert_eq!(super::net(&json!({"net_epoch": "soon"})), None);
    }

    #[test]
    fn a_launch_with_nothing_in_it_is_still_a_launch() {
        let launch = normalize(&json!({}));
        assert_eq!(at(&launch, "/title"), "");
        assert!(at(&launch, "/net_epoch").is_null());
        assert_eq!(at(&launch, "/vid_urls"), json!([]));
    }
}
