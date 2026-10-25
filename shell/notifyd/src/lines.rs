//! The daemon and the shell plugin that starts it talk in lines of JSON: the
//! plugin writes commands to its input, `{"cmd": "dismiss", "id": 4}`, and
//! reads events from its output, `{"event": "note", "id": 4, ...}`.
//!
//! Commands: `dismiss`, `hide` and `invoke` (with an `id`, and for `invoke` an
//! optional `action` key), `hideAll`, `clear`, `reshow`, `hold` and `release`
//! (the pointer over the toasts and off them), `dnd` (with `on`).
//!
//! Events: `note` (everything about one, new or replaced), `toast` (its toast
//! up or down, and its time), `closed`, `dnd`, and `bus` (whether this daemon
//! holds the notification name on the session bus).

use crate::store::{Entry, Event, Store};
use serde_json::{Value, json};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    Dismiss(u32),
    Hide(u32),
    Invoke(u32, Option<String>),
    HideAll,
    Clear,
    Reshow,
    Hold,
    Release,
    Dnd(bool),
    /// Unreadable, or unknown: ignored.
    Other(String),
}

fn id(line: &Value) -> Option<u32> {
    line.get("id")
        .and_then(Value::as_u64)
        .and_then(|n| u32::try_from(n).ok())
}

/// A command line.
#[must_use]
pub fn command(text: &str) -> Command {
    let Ok(line) = serde_json::from_str::<Value>(text) else {
        return Command::Other(text.chars().take(80).collect());
    };
    let name = line.get("cmd").and_then(Value::as_str).unwrap_or("");
    let with_id =
        |make: fn(u32) -> Command| id(&line).map_or_else(|| Command::Other(name.to_owned()), make);
    match name {
        "dismiss" => with_id(Command::Dismiss),
        "hide" => with_id(Command::Hide),
        "invoke" => id(&line).map_or_else(
            || Command::Other(name.to_owned()),
            |n| {
                Command::Invoke(
                    n,
                    line.get("action")
                        .and_then(Value::as_str)
                        .map(str::to_owned),
                )
            },
        ),
        "hideAll" => Command::HideAll,
        "clear" => Command::Clear,
        "reshow" => Command::Reshow,
        "hold" => Command::Hold,
        "release" => Command::Release,
        "dnd" => line
            .get("on")
            .and_then(Value::as_bool)
            .map_or_else(|| Command::Other(name.to_owned()), Command::Dnd),
        other => Command::Other(other.to_owned()),
    }
}

fn toast_fields(entry: &Entry, now: u64) -> Value {
    let (up, left, total, running) = entry.toast.clock(now);
    json!({ "up": up, "left": left, "total": total, "running": running })
}

/// Everything the shell shows of one.
#[must_use]
pub fn note(entry: &Entry, now: u64) -> Value {
    let note = &entry.note;
    let actions: Vec<Value> = note
        .actions
        .iter()
        .map(|action| json!({ "key": action.key, "label": action.label }))
        .collect();
    json!({
        "event": "note",
        "id": entry.id,
        "app": note.app,
        "summary": note.summary,
        "body": note.body,
        "icon": note.icon,
        "image": note.image,
        "picture": entry.picture,
        "desktopEntry": note.desktop_entry,
        "urgency": note.urgency.level(),
        "actions": actions,
        "exec": note.exec,
        "resident": note.resident,
        "transient": note.transient,
        "time": entry.time,
        "toast": toast_fields(entry, now),
    })
}

/// An event as its line, or nothing when what it names has gone since.
#[must_use]
pub fn event(store: &Store, event: &Event, now: u64) -> Option<Value> {
    match event {
        Event::Note(id) => store.get(*id).map(|entry| note(entry, now)),
        Event::Toast(id) => store.get(*id).map(|entry| {
            let mut line = toast_fields(entry, now);
            if let Some(fields) = line.as_object_mut() {
                fields.insert("event".to_owned(), json!("toast"));
                fields.insert("id".to_owned(), json!(id));
            }
            line
        }),
        Event::Closed(id) => Some(json!({ "event": "closed", "id": id })),
        Event::Dnd(on) => Some(json!({ "event": "dnd", "on": on })),
    }
}

/// Whether this daemon holds the name on the bus.
#[must_use]
pub fn bus(owner: bool) -> Value {
    json!({ "event": "bus", "owner": owner })
}

#[cfg(test)]
mod tests {
    use super::{Command, command, event};
    use crate::note::{Note, Timeout, Urgency};
    use crate::store::{Event, Store};

    #[test]
    fn reads_commands() {
        assert_eq!(
            command("{\"cmd\":\"dismiss\",\"id\":4}"),
            Command::Dismiss(4)
        );
        assert_eq!(
            command("{\"cmd\":\"invoke\",\"id\":2}"),
            Command::Invoke(2, None)
        );
        assert_eq!(
            command("{\"cmd\":\"invoke\",\"id\":2,\"action\":\"later\"}"),
            Command::Invoke(2, Some("later".to_owned()))
        );
        assert_eq!(command("{\"cmd\":\"dnd\",\"on\":true}"), Command::Dnd(true));
        assert_eq!(
            command("{\"cmd\":\"hide\"}"),
            Command::Other("hide".to_owned())
        );
        assert_eq!(command("{\"cmd\":\"hold\"}"), Command::Hold);
        assert!(matches!(command("garbage"), Command::Other(_)));
    }

    #[test]
    fn a_note_line_has_its_toast() {
        let mut store = Store::new(false);
        let asked = Note {
            app: "Vetro".to_owned(),
            summary: "Liquid glass".to_owned(),
            body: "Every surface".to_owned(),
            icon: "dev.vetro.Glass".to_owned(),
            image: String::new(),
            pixels: None,
            desktop_entry: String::new(),
            urgency: Urgency::Normal,
            actions: Vec::new(),
            exec: Vec::new(),
            resident: false,
            transient: false,
            timeout: Timeout::After(5000),
        };
        let (id, _) = store.notify(asked, 0, 1000, 7);
        let line = event(&store, &Event::Note(id), 3000).unwrap_or_default();
        assert_eq!(
            line.pointer("/summary"),
            Some(&serde_json::json!("Liquid glass"))
        );
        assert_eq!(line.pointer("/toast/left"), Some(&serde_json::json!(3000)));
        assert_eq!(line.pointer("/time"), Some(&serde_json::json!(7)));
        let toast = event(&store, &Event::Toast(id), 3000).unwrap_or_default();
        assert_eq!(toast.pointer("/event"), Some(&serde_json::json!("toast")));
        assert!(event(&store, &Event::Toast(99), 0).is_none());
    }
}
