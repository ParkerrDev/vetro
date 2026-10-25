//! Launch Library 2's side of a fetch, without the network: the address to
//! ask, and what its answer means.

use crate::cache::MAX_LAUNCHES;
use crate::config::Settings;
use crate::launch;
use serde_json::Value;

/// Seconds to wait when a rate limit does not say how long.
const THROTTLE_FALLBACK: i64 = 900;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Upcoming,
    Previous,
}

impl Kind {
    #[must_use]
    pub fn path(self) -> &'static str {
        match self {
            Kind::Upcoming => "upcoming",
            Kind::Previous => "previous",
        }
    }
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum Error {
    #[error("Launch Library rate limit, for {0}s")]
    Throttled(i64),
    #[error("Launch Library answered {0}")]
    Status(u16),
    #[error("Launch Library's answer is not the JSON it should be")]
    Json,
}

/// The address for a kind of list. One provider is `lsp__id`, several take
/// the `__in` lookup.
#[must_use]
pub fn url(settings: &Settings, kind: Kind, limit: i64) -> String {
    let ids: Vec<String> = settings.providers.iter().map(u32::to_string).collect();
    let field = if ids.len() > 1 {
        "lsp__id__in"
    } else {
        "lsp__id"
    };
    format!(
        "{}/launches/{}/?{field}={}&limit={limit}&mode=detailed",
        settings.api.trim_end_matches('/'),
        kind.path(),
        ids.join("%2C")
    )
}

/// How long a rate limit asks to wait: the number before "seconds" in its
/// message ("Expected available in 873 seconds."), or a quarter of an hour.
#[must_use]
pub fn throttle_seconds(body: &str) -> i64 {
    let Some(before) = body.find(" seconds").and_then(|at| body.get(..at)) else {
        return THROTTLE_FALLBACK;
    };
    let digits: String = before
        .chars()
        .rev()
        .take_while(char::is_ascii_digit)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();
    digits.parse().unwrap_or(THROTTLE_FALLBACK)
}

/// The launches in an answer with its HTTP status, normalised.
///
/// # Errors
/// A rate limit, any other status but 200, or a body that is not the list.
pub fn launches(status: u16, body: &str) -> Result<Vec<Value>, Error> {
    match status {
        200 => {}
        429 => return Err(Error::Throttled(throttle_seconds(body))),
        other => return Err(Error::Status(other)),
    }
    let answer: Value = serde_json::from_str(body).map_err(|_| Error::Json)?;
    let results = answer
        .get("results")
        .and_then(Value::as_array)
        .ok_or(Error::Json)?;
    Ok(results
        .iter()
        .take(MAX_LAUNCHES)
        .map(launch::normalize)
        .collect())
}

#[cfg(test)]
mod tests {
    fn at(value: &serde_json::Value, pointer: &str) -> serde_json::Value {
        value
            .pointer(pointer)
            .cloned()
            .unwrap_or(serde_json::Value::Null)
    }

    use super::{Error, Kind, launches, throttle_seconds, url};
    use crate::config::Settings;

    #[test]
    fn asks_for_spacex_by_default() {
        let address = url(&Settings::default(), Kind::Upcoming, 8);
        assert_eq!(
            address,
            "https://ll.thespacedevs.com/2.3.0/launches/upcoming/?lsp__id=121&limit=8&mode=detailed"
        );
        let several = Settings {
            providers: vec![121, 30],
            ..Settings::default()
        };
        assert!(url(&several, Kind::Previous, 5).contains("previous/?lsp__id__in=121%2C30&"));
    }

    #[test]
    fn reads_how_long_a_rate_limit_lasts() {
        assert_eq!(
            throttle_seconds(
                "{\"detail\": \"Request was throttled. Expected available in 873 seconds.\"}"
            ),
            873
        );
        assert_eq!(throttle_seconds("slow down"), 900);
        assert_eq!(
            launches(429, "available in 60 seconds"),
            Err(Error::Throttled(60))
        );
    }

    #[test]
    fn reads_the_list() {
        let body = "{\"results\": [{\"name\": \"Falcon 9 Block 5 | Bandwagon 5\"}]}";
        let list = launches(200, body).unwrap_or_default();
        assert_eq!(list.len(), 1);
        assert_eq!(
            list.first().map(|l| at(l, "/title")),
            Some(serde_json::json!("Bandwagon 5"))
        );
        assert_eq!(launches(200, "<html>"), Err(Error::Json));
        assert_eq!(launches(503, ""), Err(Error::Status(503)));
    }
}
