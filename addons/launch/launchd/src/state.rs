//! What the launch card reads from `$XDG_RUNTIME_DIR/vetro/state.json`, and the
//! control socket's requests and replies (`vetro-launch` speaks to it, one
//! JSON line each way).

use crate::cache::Cache;
use crate::config::Settings;
use serde_json::{Map, Value, json};

/// The video wallpaper's requests, which the daemon no longer answers.
const REMOVED: [&str; 12] = [
    "live",
    "idle",
    "auto",
    "mute",
    "unmute",
    "toggle-mute",
    "pause",
    "resume",
    "toggle-pause",
    "static",
    "video",
    "clip",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Request {
    Status,
    Refresh,
    Reload,
    /// One of the video wallpaper's requests.
    Removed(String),
    Unknown(String),
}

/// The state file: no wallpaper and no webcast any more, the cards' own
/// settings, and the last fetch's error.
#[must_use]
pub fn state(settings: &Settings, cache: &Cache, now: i64) -> Value {
    json!({
        "mode": "off",
        "wallpaper_mode": "static",
        "live": null,
        "muted": true,
        "overlay": settings.overlay,
        "weather": settings.weather,
        "fetch_error": cache.error,
        "updated": now,
    })
}

/// A request line: `{"cmd": "status"}`. Anything unreadable is a status.
#[must_use]
pub fn request(line: &str) -> Request {
    let parsed: Value = serde_json::from_str(line).unwrap_or(Value::Null);
    match parsed
        .get("cmd")
        .and_then(Value::as_str)
        .unwrap_or("status")
    {
        "status" => Request::Status,
        "refresh" => Request::Refresh,
        "reload" => Request::Reload,
        name if REMOVED.contains(&name) => Request::Removed(name.to_owned()),
        name => Request::Unknown(name.to_owned()),
    }
}

/// The reply to a status request: the state, and the next launch.
#[must_use]
pub fn status(settings: &Settings, cache: &Cache, now: i64) -> Value {
    let mut reply = Map::new();
    reply.insert("ok".to_owned(), Value::Bool(true));
    if let Value::Object(fields) = state(settings, cache, now) {
        reply.extend(fields);
    }
    reply.insert(
        "next".to_owned(),
        cache.next_launch(now).cloned().unwrap_or(Value::Null),
    );
    Value::Object(reply)
}

/// The reply to every request but a status.
#[must_use]
pub fn reply(request: &Request) -> Value {
    match request {
        Request::Status | Request::Refresh => {
            json!({"ok": true, "message": "refreshing launch data"})
        }
        Request::Reload => json!({"ok": true, "message": "settings read again"}),
        Request::Removed(name) => {
            json!({"ok": false, "error": format!("{name}: the video wallpaper was removed from Vetro")})
        }
        Request::Unknown(name) => {
            json!({"ok": false, "error": format!("unknown command {name:?}")})
        }
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

    use super::{Request, reply, request, status};
    use crate::cache::Cache;
    use crate::config::Settings;
    use serde_json::json;

    #[test]
    fn reads_requests() {
        assert_eq!(request("{\"cmd\": \"refresh\"}"), Request::Refresh);
        assert_eq!(
            request("{\"cmd\": \"live\", \"url\": \"x\"}"),
            Request::Removed("live".to_owned())
        );
        assert_eq!(
            request("{\"cmd\": \"fly\"}"),
            Request::Unknown("fly".to_owned())
        );
        assert_eq!(request("garbage"), Request::Status);
    }

    #[test]
    fn a_status_names_the_next_launch_and_no_wallpaper() {
        let cache = Cache {
            upcoming: vec![json!({"title": "next", "net_epoch": 50})],
            ..Cache::default()
        };
        let answer = status(&Settings::default(), &cache, 10);
        assert_eq!(at(&answer, "/ok"), true);
        assert_eq!(at(&answer, "/mode"), "off");
        assert_eq!(at(&answer, "/next/title"), "next");
        assert_eq!(at(&answer, "/overlay/corner"), "top-left");
    }

    #[test]
    fn the_wallpaper_requests_say_it_is_gone() {
        let answer = reply(&Request::Removed("static".to_owned()));
        assert_eq!(at(&answer, "/ok"), false);
        assert!(
            at(&answer, "/error")
                .as_str()
                .is_some_and(|text| text.contains("removed"))
        );
    }
}
