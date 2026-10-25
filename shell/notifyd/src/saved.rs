//! Do Not Disturb outlasts a restart of the shell (Omarchy restarts it to
//! change themes): it is kept in `~/.local/state/vetro/notifications.json` as
//! `{"doNotDisturb": true}`.

use serde_json::{Value, json};

/// Do Not Disturb in the file's text: off when the file is unreadable.
#[must_use]
pub fn dnd(text: &str) -> bool {
    serde_json::from_str::<Value>(text)
        .ok()
        .and_then(|file| file.get("doNotDisturb").and_then(Value::as_bool))
        .unwrap_or(false)
}

/// The file's text for Do Not Disturb on or off.
#[must_use]
pub fn text(dnd: bool) -> String {
    format!("{}\n", json!({ "doNotDisturb": dnd }))
}

#[cfg(test)]
mod tests {
    use super::{dnd, text};

    #[test]
    fn reads_what_it_writes() {
        assert!(dnd(&text(true)));
        assert!(!dnd(&text(false)));
        assert!(!dnd("not json"));
        assert!(!dnd("{\"doNotDisturb\": \"yes\"}"));
    }
}
