//! What is known about launches between runs: the lists, when each was
//! fetched, and why the last fetch failed. Kept in ~/.cache/vetro/launches.json,
//! which is also the file the cards read.

use crate::{clock, launch};
use serde_json::{Value, json};

/// The lists hold no more than this many launches each.
pub const MAX_LAUNCHES: usize = 100;
const POSTROLL: i64 = 20 * 60;
/// Starship webcasts run through reentry, about an hour after lift-off.
const POSTROLL_STARSHIP: i64 = 75 * 60;

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Cache {
    pub upcoming: Vec<Value>,
    pub previous: Vec<Value>,
    pub fetched_upcoming: i64,
    pub fetched_previous: i64,
    pub throttled_until: i64,
    pub error: Option<String>,
    /// Not kept in the file: after a failed fetch, the next try waits until then.
    pub retry_at: i64,
}

/// A time the Python daemon may have written as a float.
fn seconds(value: &Value, key: &str) -> i64 {
    match value.get(key) {
        Some(Value::Number(number)) => number
            .as_i64()
            .or_else(|| number.as_f64().and_then(clock::whole_seconds))
            .unwrap_or(0),
        _ => 0,
    }
}

fn list(value: &Value, key: &str) -> Vec<Value> {
    value
        .get(key)
        .and_then(Value::as_array)
        .map_or_else(Vec::new, |launches| {
            launches.iter().take(MAX_LAUNCHES).cloned().collect()
        })
}

/// How long after lift-off a launch still counts as the next one.
#[must_use]
pub fn postroll(launch: &Value) -> i64 {
    let rocket = launch.get("rocket").and_then(Value::as_str).unwrap_or("");
    if rocket.to_lowercase().contains("starship") {
        POSTROLL_STARSHIP
    } else {
        POSTROLL
    }
}

impl Cache {
    /// The cache as the file holds it; anything missing or malformed is empty.
    #[must_use]
    pub fn from_json(value: &Value) -> Self {
        Self {
            upcoming: list(value, "upcoming"),
            previous: list(value, "previous"),
            fetched_upcoming: seconds(value, "fetched_upcoming"),
            fetched_previous: seconds(value, "fetched_previous"),
            throttled_until: seconds(value, "throttled_until"),
            error: value
                .get("error")
                .and_then(Value::as_str)
                .map(str::to_owned),
            retry_at: 0,
        }
    }

    /// The file's contents at `now`.
    #[must_use]
    pub fn to_json(&self, now: i64) -> Value {
        json!({
            "provider": "Launch Library 2 (thespacedevs.com)",
            "updated": clock::format(now),
            "fetched_upcoming": self.fetched_upcoming,
            "fetched_previous": self.fetched_previous,
            "throttled_until": self.throttled_until,
            "error": self.error,
            "upcoming": self.upcoming,
            "previous": self.previous,
        })
    }

    /// The first launch still to come, or that lifted off within its postroll.
    #[must_use]
    pub fn next_launch(&self, now: i64) -> Option<&Value> {
        self.upcoming.iter().find(|candidate| {
            launch::net(candidate)
                .is_some_and(|net| net > now || now.saturating_sub(net) < postroll(candidate))
        })
    }
}

#[cfg(test)]
mod tests {
    fn at(value: &serde_json::Value, pointer: &str) -> serde_json::Value {
        value
            .pointer(pointer)
            .cloned()
            .unwrap_or(serde_json::Value::Null)
    }

    use super::{Cache, POSTROLL_STARSHIP, postroll};
    use serde_json::json;

    #[test]
    fn reads_what_the_python_daemon_wrote() {
        let file = json!({"fetched_upcoming": 1_792_400_000.5, "throttled_until": 0.0, "error": null,
                          "upcoming": [{"title": "A"}], "previous": "not a list"});
        let cache = Cache::from_json(&file);
        assert_eq!(cache.fetched_upcoming, 1_792_400_000);
        assert_eq!(cache.upcoming.len(), 1);
        assert_eq!(cache.previous, Vec::<serde_json::Value>::new());
        assert_eq!(cache.error, None);
    }

    #[test]
    fn writes_what_it_reads() {
        let cache = Cache {
            fetched_upcoming: 5,
            error: Some("down".to_owned()),
            ..Cache::default()
        };
        assert_eq!(Cache::from_json(&cache.to_json(10)), cache);
    }

    #[test]
    fn the_next_launch_stays_next_through_its_postroll() {
        let cache = Cache {
            upcoming: vec![
                json!({"title": "gone", "net_epoch": 100}),
                json!({"title": "next", "net_epoch": 5000}),
            ],
            ..Cache::default()
        };
        assert_eq!(
            cache.next_launch(200).map(|l| at(l, "/title")),
            Some(json!("gone"))
        );
        assert_eq!(
            cache.next_launch(100 + 20 * 60).map(|l| at(l, "/title")),
            Some(json!("next"))
        );
        assert_eq!(cache.next_launch(5000 + 20 * 60), None);
        assert_eq!(postroll(&json!({"rocket": "Starship"})), POSTROLL_STARSHIP);
    }
}
